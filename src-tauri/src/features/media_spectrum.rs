//! Bounded, runtime-only spectrum. No recording, files, or endpoint-wide fallback.
use serde::Serialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

pub const EVENT: &str = "vela://media-spectrum";
const N: usize = 2048;
const RATE: f32 = 44100.0;
const EDGES: [f32; 5] = [60.0, 250.0, 1000.0, 4000.0, 16000.0];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpectrumSnapshot {
    pub revision: u64,
    pub session_id: Option<String>,
    pub status: String,
    pub bands: [f32; 4],
}
impl Default for SpectrumSnapshot {
    fn default() -> Self {
        Self {
            revision: 0,
            session_id: None,
            status: "disabled".into(),
            bands: [0.0; 4],
        }
    }
}
pub fn publish(
    app: &AppHandle,
    cache: &Mutex<SpectrumSnapshot>,
    id: Option<&str>,
    status: &str,
    bands: [f32; 4],
) {
    if let Ok(mut current) = cache.lock() {
        let next = SpectrumSnapshot {
            revision: current.revision + 1,
            session_id: id.map(str::to_owned),
            status: status.into(),
            bands,
        };
        *current = next.clone();
        let _ = app.emit_to("desktop", EVENT, next);
    }
}

struct Analyzer {
    samples: Vec<[f32; 2]>,
    cursor: usize,
    window: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    levels: [f32; 4],
}
impl Analyzer {
    fn new() -> Self {
        Self {
            samples: vec![[0.0; 2]; N],
            cursor: 0,
            window: (0..N)
                .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (N - 1) as f32).cos())
                .collect(),
            re: vec![0.0; N],
            im: vec![0.0; N],
            levels: [0.0; 4],
        }
    }
    fn push(&mut self, left: f32, right: f32) {
        self.samples[self.cursor] = [
            if left.is_finite() {
                left.clamp(-1.0, 1.0)
            } else {
                0.0
            },
            if right.is_finite() {
                right.clamp(-1.0, 1.0)
            } else {
                0.0
            },
        ];
        self.cursor = (self.cursor + 1) % N;
    }
    fn analyze(&mut self) -> [f32; 4] {
        let mut power = [0.0f32; 4];
        // Transform channels separately so opposite-phase stereo cannot cancel.
        for channel in 0..2 {
            for i in 0..N {
                self.re[i] = self.samples[(self.cursor + i) % N][channel] * self.window[i];
                self.im[i] = 0.0;
            }
            fft(&mut self.re, &mut self.im);
            for bin in 1..N / 2 {
                let hz = bin as f32 * RATE / N as f32;
                if let Some(band) = (0..4).find(|&band| hz >= EDGES[band] && hz < EDGES[band + 1]) {
                    power[band] += (self.re[bin] * self.re[bin] + self.im[bin] * self.im[bin])
                        / (N * N) as f32;
                }
            }
        }
        for (band, energy) in power.iter().enumerate() {
            let db = 10.0 * (energy * 4.0).max(1e-12).log10();
            let target = ((db + 60.0) / 54.0).clamp(0.0, 1.0);
            self.levels[band] = if target > self.levels[band] {
                target
            } else {
                (self.levels[band] * 0.76).max(target)
            };
            if self.levels[band] < 0.005 {
                self.levels[band] = 0.0;
            }
        }
        self.levels
    }
}
fn fft(re: &mut [f32], im: &mut [f32]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let angle = -std::f32::consts::TAU / len as f32;
        let (sin, cos) = angle.sin_cos();
        for start in (0..n).step_by(len) {
            let (mut wr, mut wi) = (1.0, 0.0);
            for k in 0..len / 2 {
                let a = start + k;
                let b = a + len / 2;
                let tr = re[b] * wr - im[b] * wi;
                let ti = re[b] * wi + im[b] * wr;
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
                let next = wr * cos - wi * sin;
                wi = wr * sin + wi * cos;
                wr = next;
            }
        }
        len *= 2;
    }
}

#[cfg(target_os = "windows")]
pub mod native {
    use super::*;
    use std::{
        mem::{size_of, ManuallyDrop},
        sync::mpsc,
        time::{Duration, Instant},
    };
    use windows::{
        core::{implement, Interface, Ref, Result as WinResult, HRESULT, PCWSTR},
        Win32::{
            Foundation::{CloseHandle, HANDLE},
            Media::Audio::*,
            System::{
                Com::{
                    StructuredStorage::{
                        PROPVARIANT, PROPVARIANT_0, PROPVARIANT_0_0, PROPVARIANT_0_0_0,
                    },
                    BLOB,
                },
                Threading::CreateEventW,
                Variant::VT_BLOB,
            },
        },
    };

    // ActivateCompleted runs in the MTA, and the sole consumer is our MTA
    // worker. COM pointers can cross threads within that same apartment; this
    // private envelope is never consumed on an STA or shared concurrently.
    // IAudioClient has no registered agile-reference proxy on some systems.
    struct MtaClient(IAudioClient);
    unsafe impl Send for MtaClient {}

    #[implement(IActivateAudioInterfaceCompletionHandler)]
    struct Completion {
        reply: mpsc::Sender<WinResult<MtaClient>>,
        _params: Box<AUDIOCLIENT_ACTIVATION_PARAMS>,
        _variant: Box<ManuallyDrop<PROPVARIANT>>,
    }
    #[allow(non_snake_case)]
    impl IActivateAudioInterfaceCompletionHandler_Impl for Completion_Impl {
        fn ActivateCompleted(
            &self,
            operation: Ref<IActivateAudioInterfaceAsyncOperation>,
        ) -> WinResult<()> {
            let result = (|| unsafe {
                let mut hr = HRESULT(0);
                let mut object = None;
                operation.ok()?.GetActivateResult(&mut hr, &mut object)?;
                hr.ok()?;
                let client = object
                    .ok_or_else(windows::core::Error::empty)?
                    .cast::<IAudioClient>()?;
                Ok(MtaClient(client))
            })();
            let _ = self.reply.send(result);
            Ok(())
        }
    }
    struct Event(HANDLE);
    impl Drop for Event {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
    pub struct Capture {
        client: IAudioClient,
        capture: IAudioCaptureClient,
        _event: Event,
        analyzer: Analyzer,
        last_packet: Instant,
        #[cfg(test)]
        frames_read: u64,
    }
    impl Drop for Capture {
        fn drop(&mut self) {
            unsafe {
                let _ = self.client.Stop();
            }
        }
    }
    impl Capture {
        pub fn open(pid: u32) -> WinResult<Self> {
            // Enforce the same-apartment contract of MtaClient at the API entry.
            unsafe {
                use windows::Win32::System::Com::{
                    CoGetApartmentType, APTTYPE, APTTYPEQUALIFIER, APTTYPE_MTA,
                };
                let mut apartment = APTTYPE::default();
                let mut qualifier = APTTYPEQUALIFIER::default();
                CoGetApartmentType(&mut apartment, &mut qualifier)?;
                if apartment != APTTYPE_MTA {
                    return Err(windows::core::Error::from_hresult(HRESULT(
                        0x8001010Eu32 as i32,
                    )));
                }
            }
            if pid == 0 {
                return Err(windows::core::Error::from_hresult(HRESULT(
                    0x80070057u32 as i32,
                )));
            }
            let (reply, receiver) = mpsc::channel();
            let mut params = Box::new(AUDIOCLIENT_ACTIVATION_PARAMS {
                ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
                Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                    ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                        TargetProcessId: pid,
                        ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
                    },
                },
            });
            let data = (&mut *params as *mut AUDIOCLIENT_ACTIVATION_PARAMS).cast();
            // The handler owns the blob until completion, including on a timeout.
            // PROPVARIANT normally calls PropVariantClear on drop. This blob
            // borrows a Rust Box, so clearing it would incorrectly CoTaskMemFree
            // that Box. The handler owns both allocations and drops them once.
            let variant = Box::new(ManuallyDrop::new(PROPVARIANT {
                Anonymous: PROPVARIANT_0 {
                    Anonymous: ManuallyDrop::new(PROPVARIANT_0_0 {
                        vt: VT_BLOB,
                        Anonymous: PROPVARIANT_0_0_0 {
                            blob: BLOB {
                                cbSize: size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>() as u32,
                                pBlobData: data,
                            },
                        },
                        ..Default::default()
                    }),
                },
            }));
            let variant_ptr = &**variant as *const PROPVARIANT;
            let handler: IActivateAudioInterfaceCompletionHandler = Completion {
                reply,
                _params: params,
                _variant: variant,
            }
            .into();
            unsafe {
                let _operation = ActivateAudioInterfaceAsync(
                    VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
                    &IAudioClient::IID,
                    Some(variant_ptr),
                    &handler,
                )?;
                let client = receiver
                    .recv_timeout(Duration::from_secs(2))
                    .map_err(|_| {
                        windows::core::Error::from_hresult(HRESULT(0x800705B4u32 as i32))
                    })??
                    .0;
                let format = WAVEFORMATEX {
                    wFormatTag: 1,
                    nChannels: 2,
                    nSamplesPerSec: RATE as u32,
                    nAvgBytesPerSec: RATE as u32 * 4,
                    nBlockAlign: 4,
                    wBitsPerSample: 16,
                    cbSize: 0,
                };
                client.Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK
                        | AUDCLNT_STREAMFLAGS_EVENTCALLBACK
                        | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                    0,
                    0,
                    &format,
                    None,
                )?;
                let event = Event(CreateEventW(None, false, false, PCWSTR::null())?);
                client.SetEventHandle(event.0)?;
                let capture = client.GetService::<IAudioCaptureClient>()?;
                client.Start()?;
                Ok(Self {
                    client,
                    capture,
                    _event: event,
                    analyzer: Analyzer::new(),
                    last_packet: Instant::now(),
                    #[cfg(test)]
                    frames_read: 0,
                })
            }
        }
        pub fn poll(&mut self) -> WinResult<[f32; 4]> {
            unsafe {
                // Bound work if metadata loading delayed the worker; the analyzer
                // retains only its newest 2048 frames, regardless of packet size.
                for _ in 0..64 {
                    if self.capture.GetNextPacketSize()? == 0 {
                        break;
                    }
                    let mut data = std::ptr::null_mut();
                    let mut frames = 0;
                    let mut flags = 0;
                    self.capture
                        .GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;
                    if flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0 {
                        self.analyzer.samples.fill([0.0; 2]);
                    }
                    let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || data.is_null();
                    // Explicit PCM16 stereo format; unaligned reads are safe.
                    let begin = frames.saturating_sub(N as u32);
                    for i in begin..frames {
                        if silent {
                            self.analyzer.push(0.0, 0.0);
                        } else {
                            let ptr = data.add(i as usize * 4).cast::<i16>();
                            self.analyzer.push(
                                ptr.read_unaligned() as f32 / 32768.0,
                                ptr.add(1).read_unaligned() as f32 / 32768.0,
                            );
                        }
                    }
                    self.capture.ReleaseBuffer(frames)?;
                    #[cfg(test)]
                    {
                        self.frames_read += frames as u64;
                    }
                    self.last_packet = Instant::now();
                }
            }
            if self.last_packet.elapsed() > Duration::from_millis(120) {
                self.analyzer.samples.fill([0.0; 2]);
            }
            Ok(self.analyzer.analyze())
        }
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use windows::Win32::System::WinRT::{RoInitialize, RoUninitialize, RO_INIT_MULTITHREADED};
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    RoUninitialize();
                }
            }
        }
        fn initialize() -> Apartment {
            unsafe {
                RoInitialize(RO_INIT_MULTITHREADED).unwrap();
            }
            Apartment
        }
        #[test]
        #[ignore = "opens read-only Windows process loopback, no playback or UI changes"]
        fn process_loopback_can_start_stop_and_restart() {
            let _apartment = initialize();
            for _ in 0..3 {
                let mut capture =
                    Capture::open(std::process::id()).expect("process loopback activation");
                assert_eq!(capture.poll().unwrap(), [0.0; 4]);
            }
        }
        #[test]
        #[ignore = "renders all-zero PCM in the test process only; no audible sound, UI, or volume changes"]
        fn silent_process_audio_is_captured_as_real_pcm_buffers() {
            use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
            struct SilentOutput {
                client: IAudioClient,
                render: IAudioRenderClient,
            }
            impl Drop for SilentOutput {
                fn drop(&mut self) {
                    unsafe {
                        let _ = self.client.Stop();
                    }
                }
            }
            impl SilentOutput {
                fn fill(&self) {
                    unsafe {
                        let count = self.client.GetBufferSize().unwrap()
                            - self.client.GetCurrentPadding().unwrap();
                        if count > 0 {
                            self.render.GetBuffer(count).unwrap();
                            self.render
                                .ReleaseBuffer(count, AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)
                                .unwrap();
                        }
                    }
                }
            }
            let _apartment = initialize();
            let output = unsafe {
                let devices: IMMDeviceEnumerator =
                    CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).unwrap();
                let device = devices
                    .GetDefaultAudioEndpoint(eRender, eMultimedia)
                    .unwrap();
                let client: IAudioClient = device.Activate(CLSCTX_ALL, None).unwrap();
                let format = WAVEFORMATEX {
                    wFormatTag: 1,
                    nChannels: 2,
                    nSamplesPerSec: 44100,
                    nAvgBytesPerSec: 176400,
                    nBlockAlign: 4,
                    wBitsPerSample: 16,
                    cbSize: 0,
                };
                client
                    .Initialize(
                        AUDCLNT_SHAREMODE_SHARED,
                        AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                        1_000_000,
                        0,
                        &format,
                        None,
                    )
                    .unwrap();
                let render = client.GetService::<IAudioRenderClient>().unwrap();
                SilentOutput { client, render }
            };
            let mut capture = Capture::open(std::process::id()).unwrap();
            output.fill();
            unsafe {
                output.client.Start().unwrap();
            }
            for _ in 0..12 {
                std::thread::sleep(Duration::from_millis(40));
                output.fill();
                assert_eq!(capture.poll().unwrap(), [0.0; 4]);
            }
            eprintln!("Read {} real silent PCM frames", capture.frames_read);
            assert!(capture.frames_read > 0);
        }
        #[test]
        #[ignore = "reads the currently playing application's audio for one second, never changes playback"]
        fn current_player_produces_capture_buffers_when_available() {
            use crate::features::media_volume::AudioRuntime;
            use windows::Media::Control::{
                GlobalSystemMediaTransportControlsSessionManager as Manager,
                GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
            };
            let _apartment = initialize();
            let manager = Manager::RequestAsync().unwrap().join().unwrap();
            let Ok(session) = manager.GetCurrentSession() else {
                eprintln!("No current player; live audio check not exercised.");
                return;
            };
            if session.GetPlaybackInfo().unwrap().PlaybackStatus().unwrap() != Status::Playing {
                eprintln!("Current player paused; live audio check not exercised.");
                return;
            }
            let mut audio = AudioRuntime::new(std::sync::Arc::new(|_| {})).unwrap();
            audio
                .rebind(&session.SourceAppUserModelId().unwrap().to_string(), false)
                .unwrap();
            let Some(pid) = audio.spectrum_process_id() else {
                eprintln!("No unique active player process; live audio check not exercised.");
                return;
            };
            let mut capture = Capture::open(pid).unwrap();
            let mut peak = [0.0f32; 4];
            for _ in 0..25 {
                std::thread::sleep(Duration::from_millis(40));
                let bands = capture.poll().unwrap();
                for i in 0..4 {
                    peak[i] = peak[i].max(bands[i]);
                }
            }
            eprintln!(
                "Captured {} PCM frames; frequency band peaks: {:?}",
                capture.frames_read, peak
            );
            assert!(
                capture.frames_read > 0,
                "active audio produced no PCM packets"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn tone(hz: f32, opposite: bool) -> [f32; 4] {
        let mut a = Analyzer::new();
        for i in 0..N {
            let v = 0.5 * (std::f32::consts::TAU * hz * i as f32 / RATE).sin();
            a.push(v, if opposite { -v } else { v });
        }
        a.analyze()
    }
    #[test]
    fn tones_select_the_correct_logarithmic_band() {
        for (band, hz) in [120.0, 500.0, 2000.0, 8000.0].iter().enumerate() {
            let levels = tone(*hz, false);
            assert!(levels[band] > 0.8, "{levels:?}");
            for (other, level) in levels.iter().enumerate() {
                if other != band {
                    assert!(*level < 0.15, "{levels:?}");
                }
            }
        }
    }
    #[test]
    fn silence_and_nonfinite_samples_are_zero() {
        let mut a = Analyzer::new();
        for _ in 0..N {
            a.push(f32::NAN, f32::INFINITY);
        }
        assert_eq!(a.analyze(), [0.0; 4]);
    }
    #[test]
    fn opposite_phase_stereo_retains_energy() {
        let normal = tone(500.0, false);
        assert_eq!(normal, tone(500.0, true));
    }
    #[test]
    fn silence_decays_and_memory_stays_bounded() {
        let mut a = Analyzer::new();
        for i in 0..N {
            a.push((std::f32::consts::TAU * 500.0 * i as f32 / RATE).sin(), 0.0);
        }
        assert!(a.analyze()[1] > 0.8);
        for _ in 0..N * 20 {
            a.push(0.0, 0.0);
        }
        for _ in 0..30 {
            a.analyze();
        }
        assert_eq!(a.levels, [0.0; 4]);
        assert_eq!(a.samples.len(), N);
    }
}
