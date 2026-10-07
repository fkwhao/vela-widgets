//! Per-application Core Audio volume. Never falls back to endpoint/system volume.
use super::media::MediaVolume;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use windows::{
    core::{implement, Interface, Ref, Result as WinResult, BOOL, GUID, PCWSTR, PWSTR},
    Win32::{
        Foundation::{CloseHandle, PROPERTYKEY},
        Media::Audio::*,
        Storage::Packaging::Appx::GetApplicationUserModelId,
        System::{
            Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL},
            Threading::{
                OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
                PROCESS_QUERY_LIMITED_INFORMATION,
            },
        },
    },
};

#[derive(Clone, Copy)]
pub(super) enum AudioChange {
    Devices,
    Sessions,
    Volume,
}
type Callback = Arc<dyn Fn(AudioChange) + Send + Sync>;

#[implement(IAudioSessionEvents, IAudioSessionNotification, IMMNotificationClient)]
struct AudioEvents {
    changed: Callback,
}

#[allow(non_snake_case)]
impl IAudioSessionEvents_Impl for AudioEvents_Impl {
    fn OnDisplayNameChanged(&self, _: &PCWSTR, _: *const GUID) -> WinResult<()> {
        Ok(())
    }
    fn OnIconPathChanged(&self, _: &PCWSTR, _: *const GUID) -> WinResult<()> {
        Ok(())
    }
    fn OnSimpleVolumeChanged(&self, _: f32, _: BOOL, _: *const GUID) -> WinResult<()> {
        (self.changed)(AudioChange::Volume);
        Ok(())
    }
    fn OnChannelVolumeChanged(
        &self,
        _: u32,
        _: *const f32,
        _: u32,
        _: *const GUID,
    ) -> WinResult<()> {
        Ok(())
    }
    fn OnGroupingParamChanged(&self, _: *const GUID, _: *const GUID) -> WinResult<()> {
        Ok(())
    }
    fn OnStateChanged(&self, _: AudioSessionState) -> WinResult<()> {
        (self.changed)(AudioChange::Sessions);
        Ok(())
    }
    fn OnSessionDisconnected(&self, _: AudioSessionDisconnectReason) -> WinResult<()> {
        (self.changed)(AudioChange::Sessions);
        Ok(())
    }
}
#[allow(non_snake_case)]
impl IAudioSessionNotification_Impl for AudioEvents_Impl {
    fn OnSessionCreated(&self, _: Ref<IAudioSessionControl>) -> WinResult<()> {
        (self.changed)(AudioChange::Sessions);
        Ok(())
    }
}
#[allow(non_snake_case)]
impl IMMNotificationClient_Impl for AudioEvents_Impl {
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> WinResult<()> {
        (self.changed)(AudioChange::Devices);
        Ok(())
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> WinResult<()> {
        (self.changed)(AudioChange::Devices);
        Ok(())
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> WinResult<()> {
        (self.changed)(AudioChange::Devices);
        Ok(())
    }
    fn OnDefaultDeviceChanged(&self, _: EDataFlow, _: ERole, _: &PCWSTR) -> WinResult<()> {
        (self.changed)(AudioChange::Devices);
        Ok(())
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> WinResult<()> {
        Ok(())
    }
}

struct AudioDevice {
    manager: IAudioSessionManager2,
    events: IAudioSessionNotification,
}
impl Drop for AudioDevice {
    fn drop(&mut self) {
        unsafe {
            let _ = self.manager.UnregisterSessionNotification(&self.events);
        }
    }
}
struct AudioTarget {
    control: IAudioSessionControl2,
    volume: ISimpleAudioVolume,
    events: IAudioSessionEvents,
}
impl Drop for AudioTarget {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .control
                .UnregisterAudioSessionNotification(&self.events);
        }
    }
}

pub(super) struct AudioRuntime {
    enumerator: IMMDeviceEnumerator,
    events: IMMNotificationClient,
    changed: Callback,
    devices: Vec<AudioDevice>,
    targets: BTreeMap<String, AudioTarget>,
    source: String,
}
impl Drop for AudioRuntime {
    fn drop(&mut self) {
        self.targets.clear();
        self.devices.clear();
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.events);
        }
    }
}

// The strings returned by Core Audio are allocated with CoTaskMemAlloc.
fn owned_string(value: PWSTR) -> String {
    unsafe {
        let text = value.to_string().unwrap_or_default();
        CoTaskMemFree(Some(value.0.cast()));
        text
    }
}
fn process_identity(pid: u32) -> Option<(String, String)> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut path = vec![0u16; 32768];
        let mut length = path.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            PWSTR(path.as_mut_ptr()),
            &mut length,
        );
        let mut app_id = vec![0u16; 2048];
        let mut id_length = app_id.len() as u32;
        let id_result =
            GetApplicationUserModelId(handle, &mut id_length, Some(PWSTR(app_id.as_mut_ptr())));
        let _ = CloseHandle(handle);
        result.ok()?;
        let app_id = if id_result.is_ok() {
            String::from_utf16_lossy(&app_id[..id_length.saturating_sub(1) as usize])
        } else {
            String::new()
        };
        Some((String::from_utf16_lossy(&path[..length as usize]), app_id))
    }
}

fn matches_source(source: &str, path: &str, app_id: &str) -> bool {
    // Match an exact package ID, executable path, or executable filename.
    // Display names and partial strings aren't enough to select another app.
    let source = source.to_lowercase().replace('/', "\\");
    let path = path.to_lowercase().replace('/', "\\");
    if source.is_empty() {
        return false;
    }
    if source.contains('!') {
        return !app_id.is_empty() && source == app_id.to_lowercase();
    }
    if source.contains('\\') {
        return source == path;
    }
    source.ends_with(".exe") && path.rsplit('\\').next() == Some(source.as_str())
}
fn valid_level(level: f32) -> bool {
    level.is_finite() && (0.0..=100.0).contains(&level)
}

impl AudioRuntime {
    pub(super) fn new(changed: Callback) -> WinResult<Self> {
        unsafe {
            let enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let events: IMMNotificationClient = AudioEvents {
                changed: changed.clone(),
            }
            .into();
            enumerator.RegisterEndpointNotificationCallback(&events)?;
            let mut runtime = Self {
                enumerator,
                events,
                changed,
                devices: Vec::new(),
                targets: BTreeMap::new(),
                source: String::new(),
            };
            runtime.refresh_devices()?;
            Ok(runtime)
        }
    }
    fn refresh_devices(&mut self) -> WinResult<()> {
        self.targets.clear();
        self.devices.clear();
        unsafe {
            let devices = self
                .enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
            for index in 0..devices.GetCount()? {
                let device = devices.Item(index)?;
                let Ok(manager) = device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) else {
                    continue;
                };
                let events: IAudioSessionNotification = AudioEvents {
                    changed: self.changed.clone(),
                }
                .into();
                if manager.RegisterSessionNotification(&events).is_ok() {
                    // GetCount arms notifications for subsequently created sessions.
                    let binding = AudioDevice { manager, events };
                    let _ = binding.manager.GetSessionEnumerator()?.GetCount()?;
                    self.devices.push(binding);
                }
            }
        }
        Ok(())
    }
    pub(super) fn rebind(&mut self, source: &str, devices_changed: bool) -> WinResult<()> {
        if source != self.source {
            self.targets.clear();
            self.source = source.into();
        }
        if devices_changed {
            self.refresh_devices()?;
        }
        if source.is_empty() {
            self.targets.clear();
            return Ok(());
        }
        let mut candidates = BTreeMap::new();
        let mut paths = BTreeSet::new();
        unsafe {
            for (device_index, device) in self.devices.iter().enumerate() {
                let Ok(sessions) = device.manager.GetSessionEnumerator() else {
                    continue;
                };
                for index in 0..sessions.GetCount()? {
                    let Ok(control) = sessions
                        .GetSession(index)
                        .and_then(|s| s.cast::<IAudioSessionControl2>())
                    else {
                        continue;
                    };
                    if control.GetState().ok() == Some(AudioSessionStateExpired)
                        || control.IsSystemSoundsSession().0 == 0
                    {
                        continue;
                    }
                    let Some((path, app_id)) =
                        control.GetProcessId().ok().and_then(process_identity)
                    else {
                        continue;
                    };
                    if !matches_source(source, &path, &app_id) {
                        continue;
                    }
                    let Ok(volume) = control.cast::<ISimpleAudioVolume>() else {
                        continue;
                    };
                    let Ok(id) = control.GetSessionInstanceIdentifier() else {
                        continue;
                    };
                    let id = format!("{device_index}:{}", owned_string(id));
                    paths.insert(path.to_lowercase());
                    candidates.insert(id, (control, volume));
                }
            }
            // A bare filename shared by different installations is ambiguous.
            if !source.contains('!') && !source.contains(['\\', '/']) && paths.len() > 1 {
                candidates.clear();
            }
            self.targets.retain(|id, _| candidates.contains_key(id));
            for (id, (control, volume)) in candidates {
                if self.targets.contains_key(&id) {
                    continue;
                }
                let events: IAudioSessionEvents = AudioEvents {
                    changed: self.changed.clone(),
                }
                .into();
                if control.RegisterAudioSessionNotification(&events).is_ok() {
                    self.targets.insert(
                        id,
                        AudioTarget {
                            control,
                            volume,
                            events,
                        },
                    );
                }
            }
        }
        Ok(())
    }
    pub(super) fn read(&self) -> Option<MediaVolume> {
        let mut level = 0.0f32;
        let mut muted = true;
        if self.targets.is_empty() {
            return None;
        }
        for target in self.targets.values() {
            unsafe {
                level = level.max(target.volume.GetMasterVolume().ok()? * 100.0);
                muted &= target.volume.GetMute().ok()?.as_bool();
            }
        }
        Some(MediaVolume { level, muted })
    }
    pub(super) fn set(&self, level: Option<f32>, muted: Option<bool>) -> Result<(), String> {
        if level.is_some_and(|v| !valid_level(v)) || (level.is_none() && muted.is_none()) {
            return Err("音量必须在 0 到 100 之间。".into());
        }
        if self.targets.is_empty() {
            return Err("暂时无法连接当前播放器的音量。".into());
        }
        for target in self.targets.values() {
            unsafe {
                if let Some(level) = level {
                    target
                        .volume
                        .SetMasterVolume(level / 100.0, std::ptr::null())
                        .map_err(|_| "音量调整失败，请重试。")?;
                }
                if let Some(muted) = muted {
                    target
                        .volume
                        .SetMute(muted, std::ptr::null())
                        .map_err(|_| "静音操作失败，请重试。")?;
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        Media::Control::GlobalSystemMediaTransportControlsSessionManager,
        Win32::{
            Media::Audio::Endpoints::IAudioEndpointVolume,
            System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
        },
    };
    #[test]
    fn volume_matches_only_exact_player_identities() {
        assert!(matches_source("Spotify.exe", "C:\\Apps\\spotify.EXE", ""));
        assert!(matches_source(
            "C:/Apps/player.exe",
            "c:\\apps\\player.exe",
            ""
        ));
        assert!(matches_source(
            "Music.Package!Player",
            "C:\\Apps\\music.exe",
            "Music.Package!Player"
        ));
        assert!(!matches_source("Spotify", "C:\\Apps\\spotify.exe", ""));
        assert!(!matches_source(
            "player.exe",
            "C:\\Apps\\other-player.exe",
            ""
        ));
        assert!(!matches_source(
            "C:\\Apps\\player.exe",
            "D:\\Other\\player.exe",
            ""
        ));
        assert!(!matches_source(
            "Music.Package!Player",
            "C:\\Apps\\music.exe",
            "Other.Package!Player"
        ));
        assert!(!matches_source("", "C:\\Apps\\music.exe", ""));
    }
    #[test]
    fn volume_rejects_unbounded_values() {
        for level in [0.0, 35.0, 100.0] {
            assert!(valid_level(level));
        }
        for level in [-1.0, 101.0, f32::NAN, f32::INFINITY] {
            assert!(!valid_level(level));
        }
    }

    #[test]
    #[ignore = "requires the local Vela silent media test player on the desktop"]
    fn native_player_volume_round_trip_and_isolation() {
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    RoUninitialize();
                }
            }
        }
        struct Restore(Vec<(ISimpleAudioVolume, f32, bool)>);
        impl Drop for Restore {
            fn drop(&mut self) {
                for (volume, level, muted) in &self.0 {
                    unsafe {
                        let _ = volume.SetMasterVolume(*level, std::ptr::null());
                        let _ = volume.SetMute(*muted, std::ptr::null());
                    }
                }
            }
        }
        unsafe {
            RoInitialize(RO_INIT_MULTITHREADED).unwrap();
        }
        let _apartment = Apartment;
        let manager = GlobalSystemMediaTransportControlsSessionManager::RequestAsync()
            .unwrap()
            .join()
            .unwrap();
        let session = manager
            .GetCurrentSession()
            .expect("Start the local silent fixture first");
        let props = session
            .TryGetMediaPropertiesAsync()
            .unwrap()
            .join()
            .unwrap();
        assert_eq!(props.Artist().unwrap().to_string(), "Vela 验证播放器");
        assert_eq!(props.AlbumTitle().unwrap().to_string(), "本地静音测试");
        let source = session.SourceAppUserModelId().unwrap().to_string();
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut audio = AudioRuntime::new(Arc::new(move |change| {
            let _ = sender.send(change);
        }))
        .unwrap();
        audio.rebind(&source, false).unwrap();
        assert!(
            !audio.targets.is_empty(),
            "No exact audio match for {source}"
        );

        let restore = Restore(
            audio
                .targets
                .values()
                .map(|target| unsafe {
                    (
                        target.volume.clone(),
                        target.volume.GetMasterVolume().unwrap(),
                        target.volume.GetMute().unwrap().as_bool(),
                    )
                })
                .collect(),
        );
        let mut unrelated = Vec::new();
        let mut endpoints = Vec::new();
        unsafe {
            let devices = audio
                .enumerator
                .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
                .unwrap();
            for index in 0..devices.GetCount().unwrap() {
                let device = devices.Item(index).unwrap();
                if let Ok(volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) {
                    endpoints.push((
                        volume.clone(),
                        volume.GetMasterVolumeLevelScalar().unwrap(),
                        volume.GetMute().unwrap().as_bool(),
                    ));
                }
            }
            for (device_index, device) in audio.devices.iter().enumerate() {
                let sessions = device.manager.GetSessionEnumerator().unwrap();
                for index in 0..sessions.GetCount().unwrap() {
                    let control: IAudioSessionControl2 =
                        sessions.GetSession(index).unwrap().cast().unwrap();
                    let id = format!(
                        "{device_index}:{}",
                        owned_string(control.GetSessionInstanceIdentifier().unwrap())
                    );
                    if !audio.targets.contains_key(&id) {
                        let volume: ISimpleAudioVolume = control.cast().unwrap();
                        unrelated.push((
                            volume.clone(),
                            volume.GetMasterVolume().unwrap(),
                            volume.GetMute().unwrap().as_bool(),
                        ));
                    }
                }
            }
        }
        assert_eq!(manager.GetCurrentSession().unwrap(), session);
        assert!(audio.set(Some(-1.0), None).is_err());
        audio.set(Some(35.0), Some(true)).unwrap();
        let state = audio.read().unwrap();
        assert!((state.level - 35.0).abs() < 0.01 && state.muted);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        loop {
            if matches!(
                receiver
                    .recv_timeout(deadline.saturating_duration_since(std::time::Instant::now()))
                    .unwrap(),
                AudioChange::Volume
            ) {
                break;
            }
        }
        audio.set(Some(65.0), Some(false)).unwrap();
        let state = audio.read().unwrap();
        assert!((state.level - 65.0).abs() < 0.01 && !state.muted);
        for (volume, level, muted) in unrelated {
            unsafe {
                assert!((volume.GetMasterVolume().unwrap() - level).abs() < 0.0001);
                assert_eq!(volume.GetMute().unwrap().as_bool(), muted);
            }
        }
        for (volume, level, muted) in endpoints {
            unsafe {
                assert!((volume.GetMasterVolumeLevelScalar().unwrap() - level).abs() < 0.0001);
                assert_eq!(volume.GetMute().unwrap().as_bool(), muted);
            }
        }
        drop(restore);
        audio.rebind("", false).unwrap();
        assert!(audio.read().is_none());
        assert!(audio.set(Some(50.0), None).is_err());
    }
}
