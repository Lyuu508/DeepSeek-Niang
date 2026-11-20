use super::{Reply, Take, Voice, active};
use crate::{AppState, video::VideoHost};
use base64::{Engine, engine::general_purpose::STANDARD};
use companion_video::{
    CancellationToken, MAX_RECORDING_BYTES, check_recording_model, recognize_recording,
};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{Manager, State, WebviewWindow};
use windows::{
    Media::SpeechSynthesis::{SpeechSynthesizer, VoiceInformation},
    Storage::Streams::DataReader,
    Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize},
    core::{HSTRING, Result as WinResult},
};

type Recognition = (String, CancellationToken);

pub struct VoiceHost {
    scratch: PathBuf,
    tts_busy: Arc<AtomicBool>,
    recognition: Mutex<Option<Recognition>>,
    meter: Mutex<Option<(String, u8, Instant)>>,
}

impl VoiceHost {
    pub fn new(scratch: PathBuf) -> Self {
        // The application holds the database process lock.
        if let Ok(entries) = std::fs::read_dir(&scratch) {
            for entry in entries.flatten() {
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("recording-")
                    && entry
                        .file_type()
                        .is_ok_and(|kind| kind.is_dir() && !kind.is_symlink())
                {
                    let _ = std::fs::remove_dir_all(entry.path());
                }
            }
        }
        Self {
            scratch,
            tts_busy: Arc::new(AtomicBool::new(false)),
            recognition: Mutex::new(None),
            meter: Mutex::new(None),
        }
    }
}

struct WinRt;

impl WinRt {
    fn start() -> Reply<Self> {
        // Each blocking worker owns and releases its WinRT apartment.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }.map_err(|_| "VOICE_UNAVAILABLE")?;
        Ok(Self)
    }
}

impl Drop for WinRt {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

fn installed() -> WinResult<Vec<(VoiceInformation, Voice)>> {
    let voices = SpeechSynthesizer::AllVoices()?;
    (0..voices.Size()?)
        .map(|index| {
            let voice = voices.GetAt(index)?;
            let description = Voice {
                id: voice.Id()?.to_string(),
                name: voice.DisplayName()?.to_string(),
                language: voice.Language()?.to_string(),
            };
            Ok((voice, description))
        })
        .collect()
}

async fn run_synthesis<T, F>(work: F) -> Reply<T>
where
    T: Send + 'static,
    F: FnOnce() -> Reply<T> + Send + 'static,
{
    tokio::time::timeout(
        Duration::from_secs(25),
        tauri::async_runtime::spawn_blocking(move || {
            let _apartment = WinRt::start()?;
            work()
        }),
    )
    .await
    .map_err(|_| "VOICE_TIMEOUT")?
    .map_err(|_| "VOICE_UNAVAILABLE")?
}

pub async fn voices() -> Reply<Vec<Voice>> {
    run_synthesis(|| {
        Ok(installed()
            .map_err(|_| "VOICE_UNAVAILABLE")?
            .into_iter()
            .map(|(_, description)| description)
            .collect())
    })
    .await
}

struct Busy(Arc<AtomicBool>);
impl Drop for Busy {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub async fn speak(
    window: &WebviewWindow,
    id: String,
    rate: f32,
    pitch: f32,
    text: String,
) -> Reply<Take> {
    let busy = window.state::<VoiceHost>().tts_busy.clone();
    if busy.swap(true, Ordering::AcqRel) {
        return Err("VOICE_BUSY");
    }
    let slot = Busy(busy);
    run_synthesis(move || {
        let _slot = slot;
        let voices = installed().map_err(|_| "VOICE_UNAVAILABLE")?;
        let voice = if id.is_empty() {
            voices
                .into_iter()
                .find(|(_, info)| info.language.to_ascii_lowercase().starts_with("zh"))
                .map(|(voice, _)| voice)
                .or_else(|| SpeechSynthesizer::DefaultVoice().ok())
                .ok_or("VOICE_NO_OFFLINE_VOICE")?
        } else {
            voices
                .into_iter()
                .find(|(_, info)| info.id == id)
                .map(|(voice, _)| voice)
                .ok_or("VOICE_VOICE_REMOVED")?
        };
        let bytes = synthesize(&voice, rate, pitch, &text).map_err(|_| "VOICE_SYNTHESIS_FAILED")?;
        Take::from_wav(&bytes)
    })
    .await
}

fn synthesize(voice: &VoiceInformation, rate: f32, pitch: f32, text: &str) -> WinResult<Vec<u8>> {
    let synth = SpeechSynthesizer::new()?;
    synth.SetVoice(voice)?;
    let options = synth.Options()?;
    options.SetSpeakingRate(f64::from(rate))?;
    options.SetAudioPitch(f64::from(pitch))?;
    let stream = synth
        .SynthesizeTextToStreamAsync(&HSTRING::from(text))?
        .join()?;
    let size = stream.Size()?;
    if !(44..=super::MAX_WAV as u64).contains(&size) {
        return Ok(Vec::new());
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    let loaded = reader.LoadAsync(size as u32)?.join()?;
    if u64::from(loaded) != size {
        return Ok(Vec::new());
    }
    let mut bytes = vec![0; loaded as usize];
    reader.ReadBytes(&mut bytes)?;
    Ok(bytes)
}

/// Run against the installed Windows voice collection without a microphone or API credentials.
pub fn probe() -> i32 {
    let (_, wav) = companion_core::voice::guide_take();
    if Take::from_wav(&wav).is_err() {
        return 1;
    }
    let Ok(_apartment) = WinRt::start() else {
        return 2;
    };
    let Ok(voices) = installed() else { return 2 };
    let Some((voice, info)) = voices
        .iter()
        .find(|(_, info)| info.language.to_ascii_lowercase().starts_with("zh"))
        .or_else(|| voices.first())
    else {
        return 2;
    };
    let text = if info.language.starts_with("zh") {
        "语音功能测试。"
    } else {
        "This is a local speech test."
    };
    match synthesize(voice, 1.0, 1.0, text) {
        Ok(wav) if Take::from_wav(&wav).is_ok() => 0,
        _ => 3,
    }
}

struct UseRecognizer<'a>(&'a VoiceHost);
impl Drop for UseRecognizer<'_> {
    fn drop(&mut self) {
        if let Ok(mut current) = self.0.recognition.lock() {
            *current = None;
        }
    }
}

pub async fn record(
    window: &WebviewWindow,
    session: String,
    action: super::Recording,
) -> Reply<Option<String>> {
    let host = window.state::<VoiceHost>();
    if matches!(action, super::Recording::Cancel) {
        let running = host.recognition.lock().map_err(|_| "VOICE_UNAVAILABLE")?;
        if let Some((id, cancel)) = running.as_ref()
            && id == &session
        {
            cancel.cancel();
        }
        return Ok(None);
    }
    let model = window
        .state::<Arc<VideoHost>>()
        .selected_speech_model()
        .ok_or("VOICE_MODEL_REQUIRED")?;
    check_recording_model(&model).map_err(|error| error.0)?;
    let pcm = match action {
        super::Recording::Check => return Ok(None),
        super::Recording::Transcribe { pcm }
            if pcm.len() <= MAX_RECORDING_BYTES.div_ceil(3) * 4 =>
        {
            STANDARD.decode(pcm).map_err(|_| "VOICE_INVALID_AUDIO")?
        }
        _ => return Err("VOICE_INVALID_AUDIO"),
    };
    let cancel = CancellationToken::new();
    {
        let mut current = host.recognition.lock().map_err(|_| "VOICE_UNAVAILABLE")?;
        if current.is_some() {
            return Err("VOICE_BUSY");
        }
        *current = Some((session, cancel.clone()));
    }
    let _slot = UseRecognizer(&host);
    std::fs::create_dir_all(&host.scratch).map_err(|_| "VOICE_STORAGE")?;
    let scratch = tempfile::Builder::new()
        .prefix("recording-")
        .tempdir_in(&host.scratch)
        .map_err(|_| "VOICE_STORAGE")?;
    tokio::select! {
        _ = cancel.cancelled() => Err("VOICE_CANCELLED"),
        result = recognize_recording(&pcm, scratch.path(), &model) => {
            result.map(Some).map_err(|error| error.0)
        }
    }
}

#[tauri::command]
pub fn voice_meter(
    window: WebviewWindow,
    state: State<'_, AppState>,
    host: State<'_, VoiceHost>,
    persona_id: String,
    level: u8,
) -> Reply<()> {
    drop(active(&window, &state, &persona_id)?);
    *host.meter.lock().map_err(|_| "VOICE_UNAVAILABLE")? =
        Some((persona_id, level.min(100), Instant::now()));
    Ok(())
}

#[tauri::command]
pub fn pet_speech_level(
    window: WebviewWindow,
    host: State<'_, VoiceHost>,
    persona_id: String,
) -> Reply<u8> {
    if window.label() != "pet" {
        return Err("VOICE_UNAVAILABLE");
    }
    let pulse = host.meter.lock().map_err(|_| "VOICE_UNAVAILABLE")?;
    Ok(pulse
        .as_ref()
        .filter(|(persona, _, when)| {
            persona == &persona_id && when.elapsed() < Duration::from_millis(450)
        })
        .map_or(0, |(_, level, _)| *level))
}
