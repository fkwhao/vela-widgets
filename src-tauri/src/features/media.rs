//! Runtime-only SMTC state. Metadata and artwork never enter SQLite.
use serde::{Deserialize, Serialize};
use std::sync::{mpsc, Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

const UPDATED_EVENT: &str = "vela://media-updated";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSettings {
    #[serde(default)]
    pub theme: MediaTheme,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MediaTheme {
    #[default]
    Default,
    Vinyl,
    Atmosphere,
    Cream,
    Cassette,
    Minimal,
}

#[tauri::command]
pub async fn set_media_theme(
    app: AppHandle,
    state: State<'_, crate::AppState>,
    theme: MediaTheme,
) -> Result<crate::AppSnapshot, String> {
    let mut dimensions = (364.0, 170.0);
    let mut corner_radius = 19;
    crate::update_settings(state.inner(), |settings| {
        settings.media.theme = theme;
        let widget = settings
            .widgets
            .get_mut("media")
            .ok_or("找不到播放器组件的设置。")?;
        dimensions = widget_dimensions(&widget.size, &settings.media.theme);
        (widget.width, widget.height) = dimensions;
        corner_radius = settings.widget_corner_radius;
        Ok(())
    })?;
    crate::resize_widget_window(&app, "media", dimensions.0, dimensions.1, corner_radius)?;
    crate::publish_snapshot(&app, state.inner())
}

pub fn widget_dimensions(size: &str, theme: &MediaTheme) -> (f64, f64) {
    if size == "medium" && matches!(theme, MediaTheme::Atmosphere) {
        (170.0, 364.0)
    } else {
        crate::widget_dimensions(size)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaControls {
    play: bool,
    pause: bool,
    previous: bool,
    next: bool,
    seek: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSession {
    id: String,
    source: String,
    title: String,
    artist: String,
    album: String,
    artwork: Option<String>,
    playback_status: String,
    position_ms: i64,
    duration_ms: i64,
    updated_at: i64,
    playback_rate: f64,
    controls: MediaControls,
    volume: Option<MediaVolume>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct MediaVolume {
    pub(super) level: f32,
    pub(super) muted: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSnapshot {
    revision: u64,
    status: String,
    session: Option<MediaSession>,
    error: Option<String>,
}

impl MediaSnapshot {
    fn empty(status: &str) -> Self {
        Self {
            revision: 0,
            status: status.into(),
            session: None,
            error: None,
        }
    }
    fn unavailable(message: &str) -> Self {
        Self {
            error: Some(message.into()),
            ..Self::empty("unavailable")
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaAction {
    session_id: String,
    action: String,
    position_ms: Option<i64>,
    volume_level: Option<f32>,
    muted: Option<bool>,
}

enum Request {
    Enable(bool),
    Refresh,
    Changed,
    Control(MediaAction, mpsc::Sender<Result<(), String>>),
}

pub struct MediaState {
    sender: mpsc::Sender<Request>,
    snapshot: Arc<Mutex<MediaSnapshot>>,
}

fn publish(app: &AppHandle, cache: &Mutex<MediaSnapshot>, mut next: MediaSnapshot) {
    if let Ok(mut current) = cache.lock() {
        next.revision = current.revision + 1;
        *current = next.clone();
        // Cover data is delivered only to this widget, not to every WebView.
        let _ = app.emit_to("media", UPDATED_EVENT, next);
    }
}

pub fn start_worker(app: AppHandle, enabled: bool) {
    let (sender, receiver) = mpsc::channel();
    let snapshot = Arc::new(Mutex::new(MediaSnapshot::empty("disabled")));
    app.manage(MediaState {
        sender: sender.clone(),
        snapshot: snapshot.clone(),
    });
    let _ = sender.send(Request::Enable(enabled));
    std::thread::spawn(move || {
        #[cfg(target_os = "windows")]
        native::run(app, snapshot, sender, receiver);
        #[cfg(not(target_os = "windows"))]
        {
            let _ = sender;
            while let Ok(request) = receiver.recv() {
                match request {
                    Request::Enable(on) => publish(
                        &app,
                        &snapshot,
                        if on {
                            MediaSnapshot::unavailable("正在播放需要 Windows 10 1809 或更新版本。")
                        } else {
                            MediaSnapshot::empty("disabled")
                        },
                    ),
                    Request::Control(_, reply) => {
                        let _ = reply.send(Err("当前系统不支持媒体控制。".into()));
                    }
                    _ => {}
                }
            }
        }
    });
}

pub fn set_enabled(app: &AppHandle, enabled: bool) {
    let _ = app
        .state::<MediaState>()
        .sender
        .send(Request::Enable(enabled));
}

#[tauri::command]
pub fn get_media_snapshot(state: State<'_, MediaState>) -> Result<MediaSnapshot, String> {
    state
        .snapshot
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "暂时无法读取媒体信息。".into())
}

#[tauri::command]
pub fn refresh_media(state: State<'_, MediaState>) -> Result<(), String> {
    state
        .sender
        .send(Request::Refresh)
        .map_err(|_| "媒体服务暂时不可用，请重新启动 Vela。".into())
}

#[tauri::command]
pub async fn media_action(state: State<'_, MediaState>, input: MediaAction) -> Result<(), String> {
    let (reply, result) = mpsc::channel();
    state
        .sender
        .send(Request::Control(input, reply))
        .map_err(|_| "媒体服务暂时不可用。".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        result
            .recv_timeout(std::time::Duration::from_secs(8))
            .map_err(|_| "播放器没有响应，请稍后重试。".to_string())?
    })
    .await
    .map_err(|_| "媒体操作未完成。".to_string())?
}

#[cfg(target_os = "windows")]
mod native {
    use super::super::media_volume::{AudioChange, AudioRuntime};
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::sync::atomic::{AtomicU8, Ordering};
    use windows::{
        core::Result as WinResult,
        Foundation::TypedEventHandler,
        Media::Control::{
            CurrentSessionChangedEventArgs, GlobalSystemMediaTransportControlsSession as Session,
            GlobalSystemMediaTransportControlsSessionManager as SessionManager,
            GlobalSystemMediaTransportControlsSessionMediaProperties as Properties,
            GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
            MediaPropertiesChangedEventArgs, PlaybackInfoChangedEventArgs,
            SessionsChangedEventArgs, TimelinePropertiesChangedEventArgs,
        },
        Storage::Streams::{Buffer, DataReader, InputStreamOptions},
        Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED},
    };

    const STATE: u8 = 1;
    const METADATA: u8 = 2;
    const CURRENT: u8 = 4;
    const AUDIO: u8 = 8;
    const VOLUME: u8 = 16;
    const DEVICES: u8 = 32;
    const MAX_ARTWORK_BYTES: u64 = 2 * 1024 * 1024;
    const WINDOWS_EPOCH_MS: i64 = 11_644_473_600_000;

    struct Apartment;
    impl Drop for Apartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    // Drop removes every token, including partially registered subscriptions.
    struct ManagerSubscription {
        value: SessionManager,
        current: Option<i64>,
        sessions: Option<i64>,
    }
    impl Drop for ManagerSubscription {
        fn drop(&mut self) {
            if let Some(token) = self.current {
                let _ = self.value.RemoveCurrentSessionChanged(token);
            }
            if let Some(token) = self.sessions {
                let _ = self.value.RemoveSessionsChanged(token);
            }
        }
    }
    struct SessionSubscription {
        value: Session,
        id: String,
        metadata: Option<i64>,
        playback: Option<i64>,
        timeline: Option<i64>,
    }
    impl Drop for SessionSubscription {
        fn drop(&mut self) {
            if let Some(token) = self.metadata {
                let _ = self.value.RemoveMediaPropertiesChanged(token);
            }
            if let Some(token) = self.playback {
                let _ = self.value.RemovePlaybackInfoChanged(token);
            }
            if let Some(token) = self.timeline {
                let _ = self.value.RemoveTimelinePropertiesChanged(token);
            }
        }
    }

    #[derive(Clone)]
    struct Signal {
        sender: mpsc::Sender<Request>,
        pending: Arc<AtomicU8>,
    }
    impl Signal {
        fn changed(&self, flags: u8) {
            // Coalesce bursts without growing a queue of artwork reads.
            if self.pending.fetch_or(flags, Ordering::SeqCst) == 0 {
                let _ = self.sender.send(Request::Changed);
            }
        }
    }

    fn subscribe_manager(signal: &Signal) -> WinResult<ManagerSubscription> {
        let mut sub = ManagerSubscription {
            value: SessionManager::RequestAsync()?.join()?,
            current: None,
            sessions: None,
        };
        let events = signal.clone();
        sub.current = Some(sub.value.CurrentSessionChanged(&TypedEventHandler::<
            SessionManager,
            CurrentSessionChangedEventArgs,
        >::new(move |_, _| {
            events.changed(CURRENT | METADATA);
            Ok(())
        }))?);
        let events = signal.clone();
        sub.sessions = Some(sub.value.SessionsChanged(&TypedEventHandler::<
            SessionManager,
            SessionsChangedEventArgs,
        >::new(move |_, _| {
            events.changed(CURRENT | METADATA);
            Ok(())
        }))?);
        Ok(sub)
    }

    fn subscribe_session(
        value: Session,
        id: String,
        signal: &Signal,
    ) -> WinResult<SessionSubscription> {
        let mut sub = SessionSubscription {
            value,
            id,
            metadata: None,
            playback: None,
            timeline: None,
        };
        let events = signal.clone();
        sub.metadata = Some(sub.value.MediaPropertiesChanged(&TypedEventHandler::<
            Session,
            MediaPropertiesChangedEventArgs,
        >::new(move |_, _| {
            events.changed(METADATA);
            Ok(())
        }))?);
        let events = signal.clone();
        sub.playback = Some(sub.value.PlaybackInfoChanged(&TypedEventHandler::<
            Session,
            PlaybackInfoChangedEventArgs,
        >::new(move |_, _| {
            events.changed(STATE);
            Ok(())
        }))?);
        let events = signal.clone();
        sub.timeline =
            Some(sub.value.TimelinePropertiesChanged(&TypedEventHandler::<
                Session,
                TimelinePropertiesChangedEventArgs,
            >::new(move |_, _| {
                events.changed(STATE);
                Ok(())
            }))?);
        Ok(sub)
    }

    fn current_session(manager: &SessionManager) -> WinResult<Option<Session>> {
        match manager.GetCurrentSession() {
            Ok(session) => Ok(Some(session)),
            Err(error) if error.code() == windows::core::HRESULT(0x80004003u32 as i32) => {
                // A null WinRT object maps to E_POINTER. Fall back only when
                // Windows hasn't nominated a session; other failures are shown.
                let sessions = manager.GetSessions()?;
                let mut first = None;
                for i in 0..sessions.Size()? {
                    let session = sessions.GetAt(i)?;
                    if session.GetPlaybackInfo()?.PlaybackStatus()? == PlaybackStatus::Playing {
                        return Ok(Some(session));
                    }
                    if first.is_none() {
                        first = Some(session);
                    }
                }
                Ok(first)
            }
            Err(error) => Err(error),
        }
    }

    fn now_ms() -> i64 {
        chrono::Utc::now().timestamp_millis()
    }
    fn text(value: WinResult<windows::core::HSTRING>) -> String {
        value
            .map(|v| v.to_string().chars().take(300).collect())
            .unwrap_or_default()
    }
    fn read_artwork(properties: &Properties) -> Option<String> {
        let stream = properties
            .Thumbnail()
            .ok()?
            .OpenReadAsync()
            .ok()?
            .join()
            .ok()?;
        let result = (|| {
            let size = stream.Size().ok()?;
            if size == 0 || size > MAX_ARTWORK_BYTES {
                return None;
            }
            let buffer = Buffer::Create(size as u32).ok()?;
            let buffer = stream
                .ReadAsync(&buffer, size as u32, InputStreamOptions::None)
                .ok()?
                .join()
                .ok()?;
            let mut bytes = vec![0; buffer.Length().ok()? as usize];
            let reader = DataReader::FromBuffer(&buffer).ok()?;
            reader.ReadBytes(&mut bytes).ok()?;
            let _ = reader.Close();
            // Inspect the bytes rather than trusting the player's MIME type.
            let mime = artwork_mime(&bytes)?;
            Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
        })();
        let _ = stream.Close();
        result
    }

    fn artwork_mime(bytes: &[u8]) -> Option<&'static str> {
        if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some("image/png")
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            Some("image/jpeg")
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some("image/gif")
        } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            Some("image/webp")
        } else {
            None
        }
    }

    fn read_session(
        sub: &SessionSubscription,
        previous: Option<&MediaSession>,
        metadata: bool,
    ) -> WinResult<MediaSession> {
        let info = sub.value.GetPlaybackInfo()?;
        let status = info.PlaybackStatus()?;
        let playing = status == PlaybackStatus::Playing;
        let controls = info.Controls()?;
        let mut next = previous
            .filter(|s| s.id == sub.id)
            .cloned()
            .unwrap_or(MediaSession {
                id: sub.id.clone(),
                source: text(sub.value.SourceAppUserModelId()),
                title: String::new(),
                artist: String::new(),
                album: String::new(),
                artwork: None,
                playback_status: String::new(),
                position_ms: 0,
                duration_ms: 0,
                updated_at: now_ms(),
                playback_rate: 1.0,
                controls: MediaControls::default(),
                volume: None,
            });
        if metadata || previous.is_none_or(|s| s.id != sub.id) {
            // A newly changed track must not retain the previous track's cover.
            let props = sub.value.TryGetMediaPropertiesAsync()?.join()?;
            next.title = text(props.Title());
            next.artist = text(props.Artist());
            next.album = text(props.AlbumTitle());
            next.artwork = read_artwork(&props);
        }
        next.playback_status = if playing {
            "playing"
        } else if status == PlaybackStatus::Paused {
            "paused"
        } else if status == PlaybackStatus::Stopped {
            "stopped"
        } else {
            "closed"
        }
        .into();
        next.playback_rate = info
            .PlaybackRate()
            .and_then(|r| r.Value())
            .ok()
            .filter(|r| r.is_finite() && *r > 0.0 && *r <= 16.0)
            .unwrap_or(1.0);
        next.controls = MediaControls {
            play: controls.IsPlayEnabled().unwrap_or(false)
                || controls.IsPlayPauseToggleEnabled().unwrap_or(false),
            pause: controls.IsPauseEnabled().unwrap_or(false)
                || controls.IsPlayPauseToggleEnabled().unwrap_or(false),
            previous: controls.IsPreviousEnabled().unwrap_or(false),
            next: controls.IsNextEnabled().unwrap_or(false),
            seek: controls.IsPlaybackPositionEnabled().unwrap_or(false),
        };
        // Timeline is optional (for example, a live stream).
        next.position_ms = 0;
        next.duration_ms = 0;
        next.updated_at = now_ms();
        if let Ok(timeline) = sub.value.GetTimelineProperties() {
            let start = timeline
                .StartTime()
                .map(|t| t.Duration / 10_000)
                .unwrap_or(0);
            let end = timeline.EndTime().map(|t| t.Duration / 10_000).unwrap_or(0);
            next.duration_ms = end.saturating_sub(start).max(0);
            next.position_ms = (timeline.Position()?.Duration / 10_000)
                .saturating_sub(start)
                .clamp(0, next.duration_ms);
            let timestamp = timeline
                .LastUpdatedTime()
                .map(|t| t.UniversalTime / 10_000 - WINDOWS_EPOCH_MS)
                .unwrap_or(now_ms());
            // Some players omit LastUpdatedTime. Avoid projecting from 1601.
            next.updated_at = if timestamp > 0 && timestamp <= now_ms() {
                timestamp
            } else {
                now_ms()
            };
        }
        Ok(next)
    }

    fn control(sub: &SessionSubscription, input: &MediaAction) -> Result<(), String> {
        if input.session_id != sub.id {
            return Err("播放来源已切换，请重试。".into());
        }
        let info = sub
            .value
            .GetPlaybackInfo()
            .map_err(|_| "播放器已关闭。".to_string())?;
        let controls = info
            .Controls()
            .map_err(|_| "播放器没有提供控制权限。".to_string())?;
        let result = match input.action.as_str() {
            "play" if controls.IsPlayEnabled().unwrap_or(false) => sub.value.TryPlayAsync(),
            "pause" if controls.IsPauseEnabled().unwrap_or(false) => sub.value.TryPauseAsync(),
            "play" | "pause" if controls.IsPlayPauseToggleEnabled().unwrap_or(false) => {
                // Explicit desired state avoids double toggles on delayed events.
                let playing = info.PlaybackStatus().ok() == Some(PlaybackStatus::Playing);
                if playing == (input.action == "play") {
                    return Ok(());
                }
                sub.value.TryTogglePlayPauseAsync()
            }
            "previous" if controls.IsPreviousEnabled().unwrap_or(false) => {
                sub.value.TrySkipPreviousAsync()
            }
            "next" if controls.IsNextEnabled().unwrap_or(false) => sub.value.TrySkipNextAsync(),
            "seek" if controls.IsPlaybackPositionEnabled().unwrap_or(false) => {
                let position = input.position_ms.ok_or("请选择播放位置。")?;
                let timeline = sub
                    .value
                    .GetTimelineProperties()
                    .map_err(|_| "播放器没有提供进度。")?;
                let start = timeline
                    .StartTime()
                    .map_err(|_| "无法读取播放起点。")?
                    .Duration;
                let end = timeline
                    .EndTime()
                    .map_err(|_| "无法读取播放终点。")?
                    .Duration;
                if position < 0 || end <= start || position > (end - start) / 10_000 {
                    return Err("播放位置超出范围。".into());
                }
                let ticks = position
                    .checked_mul(10_000)
                    .and_then(|p| start.checked_add(p))
                    .ok_or("播放位置超出范围。")?;
                sub.value.TryChangePlaybackPositionAsync(ticks)
            }
            _ => return Err("播放器不支持这个操作。".into()),
        };
        match result.and_then(|operation| operation.join()) {
            Ok(true) => Ok(()),
            _ => Err("播放器未接受这个操作，请稍后重试。".into()),
        }
    }

    pub(super) fn run(
        app: AppHandle,
        cache: Arc<Mutex<MediaSnapshot>>,
        sender: mpsc::Sender<Request>,
        receiver: mpsc::Receiver<Request>,
    ) {
        let apartment = unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map(|_| Apartment);
        let signal = Signal {
            sender,
            pending: Arc::new(AtomicU8::new(0)),
        };
        let mut enabled = false;
        let mut manager: Option<ManagerSubscription> = None;
        let mut session: Option<SessionSubscription> = None;
        let mut audio: Option<AudioRuntime> = None;
        let mut generation = 0u64;
        while let Ok(request) = receiver.recv() {
            let mut flags = 0;
            match request {
                Request::Enable(on) => {
                    enabled = on;
                    if !on {
                        session = None;
                        manager = None;
                        audio = None;
                        publish(&app, &cache, MediaSnapshot::empty("disabled"));
                        continue;
                    }
                    flags = CURRENT | METADATA;
                }
                Request::Refresh if enabled => {
                    flags = CURRENT | METADATA;
                }
                Request::Changed => {
                    flags = signal.pending.swap(0, Ordering::SeqCst);
                }
                Request::Control(input, reply) => {
                    let metadata = matches!(input.action.as_str(), "previous" | "next");
                    let result = if !enabled {
                        Err("请先开启正在播放组件。".into())
                    } else if let (Some(sub), Some(manager)) = (&session, &manager) {
                        // Validate against Windows' current session, not merely our cached UI.
                        match current_session(&manager.value) {
                            Ok(Some(current)) if current == sub.value => {
                                if input.session_id != sub.id {
                                    Err("播放来源已切换，请重试。".into())
                                } else if matches!(input.action.as_str(), "volume" | "mute") {
                                    if let Some(audio) = audio.as_mut() {
                                        let source = text(sub.value.SourceAppUserModelId());
                                        if audio.rebind(&source, false).is_err() {
                                            Err("暂时无法连接当前播放器的音量。".into())
                                        } else if input.action == "volume" {
                                            input
                                                .volume_level
                                                .ok_or_else(|| "请选择音量。".into())
                                                .and_then(|level| {
                                                    audio.set(
                                                        Some(level),
                                                        (level > 0.0).then_some(false),
                                                    )
                                                })
                                        } else {
                                            input
                                                .muted
                                                .ok_or_else(|| "请选择静音状态。".into())
                                                .and_then(|muted| audio.set(None, Some(muted)))
                                        }
                                    } else {
                                        Err("暂时无法连接当前播放器的音量。".into())
                                    }
                                } else {
                                    control(sub, &input)
                                }
                            }
                            _ => Err("播放来源已切换，请重试。".into()),
                        }
                    } else {
                        Err("当前没有可控制的媒体。".into())
                    };
                    let _ = reply.send(result);
                    flags = CURRENT | STATE | if metadata { METADATA } else { 0 };
                }
                _ => {}
            }
            if !enabled || flags == 0 {
                continue;
            }
            if apartment.is_err() {
                publish(
                    &app,
                    &cache,
                    MediaSnapshot::unavailable("Windows 媒体服务暂时不可用。"),
                );
                continue;
            }
            let result: WinResult<MediaSnapshot> = (|| {
                if manager.is_none() {
                    manager = Some(subscribe_manager(&signal)?);
                    flags |= CURRENT | METADATA;
                }
                if audio.is_none() {
                    let events = signal.clone();
                    audio = AudioRuntime::new(Arc::new(move |change| {
                        events.changed(match change {
                            AudioChange::Devices => DEVICES | AUDIO,
                            AudioChange::Sessions => AUDIO,
                            AudioChange::Volume => VOLUME,
                        })
                    }))
                    .ok();
                    flags |= AUDIO;
                }
                if flags & CURRENT != 0 {
                    let current = current_session(&manager.as_ref().unwrap().value)?;
                    let unchanged = session
                        .as_ref()
                        .is_some_and(|sub| current.as_ref() == Some(&sub.value));
                    if !unchanged {
                        session = None;
                        if let Some(current) = current {
                            generation += 1;
                            session =
                                Some(subscribe_session(current, generation.to_string(), &signal)?);
                            flags |= METADATA;
                        }
                    }
                }
                let previous = cache.lock().ok().and_then(|s| s.session.clone());
                let mut item = session
                    .as_ref()
                    .map(|sub| read_session(sub, previous.as_ref(), flags & METADATA != 0))
                    .transpose()?;
                if let Some(audio) = audio.as_mut() {
                    let source = item.as_ref().map(|s| s.source.as_str()).unwrap_or_default();
                    if flags & (CURRENT | AUDIO | DEVICES) != 0
                        && audio.rebind(source, flags & DEVICES != 0).is_err()
                    {
                        // An audio-device failure doesn't hide the track or playback controls.
                        audio.rebind("", false).ok();
                    }
                    if let Some(item) = item.as_mut() {
                        item.volume = audio.read();
                    }
                } else if let Some(item) = item.as_mut() {
                    item.volume = None;
                }
                Ok(MediaSnapshot {
                    session: item,
                    ..MediaSnapshot::empty("ready")
                })
            })();
            match result {
                Ok(snapshot) => publish(&app, &cache, snapshot),
                Err(_) => {
                    session = None;
                    manager = None;
                    audio = None;
                    publish(
                        &app,
                        &cache,
                        MediaSnapshot::unavailable("暂时无法读取媒体信息，点击重试重新连接。"),
                    );
                }
            }
        }
        // COM objects must be released before RoUninitialize.
        drop(session);
        drop(manager);
        drop(audio);
        drop(apartment);
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        #[test]
        fn artwork_accepts_only_recognized_raster_formats() {
            assert_eq!(artwork_mime(b"\x89PNG\r\n\x1a\n"), Some("image/png"));
            assert_eq!(artwork_mime(&[0xff, 0xd8, 0xff]), Some("image/jpeg"));
            assert_eq!(artwork_mime(b"RIFF1234WEBP"), Some("image/webp"));
            assert_eq!(artwork_mime(b"GIF89a"), Some("image/gif"));
            assert_eq!(artwork_mime(b"<svg onload='x'>"), None);
            assert_eq!(artwork_mime(b""), None);
        }
        #[test]
        #[ignore = "requires a running Windows media service"]
        fn native_media_api_connects_and_subscriptions_can_be_removed() {
            // Read-only integration smoke test on Windows; no user media is controlled.
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.unwrap();
            let apartment = Apartment;
            let (sender, _receiver) = mpsc::channel();
            let signal = Signal {
                sender,
                pending: Arc::new(AtomicU8::new(0)),
            };
            let manager = subscribe_manager(&signal).unwrap();
            let _ = current_session(&manager.value).unwrap();
            drop(manager);
            drop(apartment);
        }

        #[test]
        #[ignore = "requires the local Vela silent media test player"]
        fn native_media_controls_and_events_round_trip() {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.unwrap();
            let apartment = Apartment;
            let (sender, receiver) = mpsc::channel();
            let signal = Signal {
                sender,
                pending: Arc::new(AtomicU8::new(0)),
            };
            let manager = subscribe_manager(&signal).unwrap();
            let current = current_session(&manager.value)
                .unwrap()
                .expect("Start the Vela silent test player first");
            let sub = subscribe_session(current, "test-session".into(), &signal).unwrap();
            assert_eq!(
                current_session(&manager.value).unwrap().as_ref(),
                Some(&sub.value)
            );
            // Chromium can register the audio session before publishing its
            // metadata. Wait for the fixture identity before issuing controls.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let initial = loop {
                let item = read_session(&sub, None, true).unwrap();
                if item.artist == "Vela 验证播放器" && item.album == "本地静音测试" {
                    break item;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "Start the Vela silent test player first"
                );
                signal.pending.store(0, Ordering::SeqCst);
                let _ = receiver.recv_timeout(std::time::Duration::from_millis(200));
            };
            // Never send test controls to a user's unrelated media session.
            assert_eq!(initial.artist, "Vela 验证播放器");
            assert_eq!(initial.album, "本地静音测试");
            assert!(initial
                .artwork
                .as_deref()
                .is_some_and(|s| s.starts_with("data:image/png;base64,")));
            assert!(
                initial.controls.play
                    && initial.controls.pause
                    && initial.controls.previous
                    && initial.controls.next
                    && initial.controls.seek
            );
            assert!(initial.duration_ms > 0);
            let action = |name: &str, position| MediaAction {
                session_id: sub.id.clone(),
                action: name.into(),
                position_ms: position,
                volume_level: None,
                muted: None,
            };
            let wait_for = |predicate: &dyn Fn(&MediaSession) -> bool| {
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                loop {
                    let next = read_session(&sub, None, true).unwrap();
                    if predicate(&next) {
                        return next;
                    }
                    assert!(
                        std::time::Instant::now() < deadline,
                        "Player did not publish the expected state"
                    );
                    signal.pending.store(0, Ordering::SeqCst);
                    let _ = receiver.recv_timeout(std::time::Duration::from_millis(200));
                }
            };
            let mut stale = action("pause", None);
            stale.session_id = "previous-session".into();
            assert!(control(&sub, &stale).is_err());
            assert!(control(&sub, &action("seek", Some(-1))).is_err());
            assert!(control(&sub, &action("seek", Some(initial.duration_ms + 1))).is_err());
            control(&sub, &action("pause", None)).unwrap();
            wait_for(&|s| s.playback_status == "paused");
            control(&sub, &action("next", None)).unwrap();
            let next = wait_for(&|s| s.title != initial.title);
            control(&sub, &action("previous", None)).unwrap();
            wait_for(&|s| s.title == initial.title && s.title != next.title);
            control(&sub, &action("seek", Some(30_000))).unwrap();
            wait_for(&|s| (s.position_ms - 30_000).abs() < 1500);
            control(&sub, &action("play", None)).unwrap();
            wait_for(&|s| s.playback_status == "playing");
            control(&sub, &action("pause", None)).unwrap();
            wait_for(&|s| s.playback_status == "paused");
            drop(sub);
            drop(manager);
            drop(apartment);
        }
    }
}
