//! Local media execution. No account access, model downloads, or external media transport.
mod media;
mod model;

use companion_core::{
    Error as CoreError,
    companion::excerpt,
    storage::Store,
    task::Lease,
    video::{LocalMedia, SegmentAnalysis, VideoModels, VideoWork},
};
use media::Decoder;
use model::Ollama;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
pub use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug)]
pub struct Failure(pub &'static str);
pub type Outcome<T> = Result<T, Failure>;

impl From<CoreError> for Failure {
    fn from(error: CoreError) -> Self {
        Self(match error {
            CoreError::Invalid("video daily limit") => "VIDEO_DAILY_LIMIT",
            CoreError::Untrusted => "VIDEO_PERSONA_CHANGED",
            CoreError::StaleLease => "VIDEO_TASK_STATE",
            CoreError::Conflict => "VIDEO_BUSY_OR_CHANGED",
            _ => "VIDEO_STORAGE_ERROR",
        })
    }
}

const MAX_WHISPER_BYTES: u64 = 2 * 1024 * 1024 * 1024;

pub fn supported_model(model: &str) -> bool {
    matches!(model, "qwen3-vl:2b" | "qwen3-vl:4b" | "qwen3-vl:8b")
}

pub async fn prepare_video(
    path: PathBuf,
    visual: String,
    speech: Option<PathBuf>,
) -> Outcome<(LocalMedia, VideoModels)> {
    if !supported_model(&visual) {
        return Err(Failure("VIDEO_MODEL_INVALID"));
    }
    let decoder = Decoder::installed().await?;
    let media = decoder.probe(path).await?;
    let ollama = Ollama::local()?;
    let visual_digest = ollama.inspect(&visual).await?;
    let (whisper_path, whisper_digest) = if media.has_audio {
        let model = speech.ok_or(Failure("VIDEO_SPEECH_MODEL_REQUIRED"))?;
        media::check_whisper()?;
        let (digest, _) = media::fingerprint(model.clone(), MAX_WHISPER_BYTES).await?;
        let saved = model
            .into_os_string()
            .into_string()
            .map_err(|_| Failure("VIDEO_SPEECH_MODEL_REQUIRED"))?;
        (Some(saved), Some(digest))
    } else {
        (None, None)
    };
    Ok((
        media,
        VideoModels {
            visual,
            visual_digest,
            whisper_path,
            whisper_digest,
            decoder_revision: decoder.revision,
        },
    ))
}

pub struct VideoRunner {
    pub store: Arc<Mutex<Store>>,
    root: PathBuf,
    clock: Instant,
}

impl VideoRunner {
    pub fn new(store: Arc<Mutex<Store>>, root: PathBuf, clock: Instant) -> Self {
        Self { store, root, clock }
    }

    pub fn tick(&self) -> i64 {
        self.clock.elapsed().as_millis().min(i64::MAX as u128) as i64
    }

    fn storage<T>(&self, f: impl FnOnce(&mut Store) -> companion_core::Result<T>) -> Outcome<T> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| Failure("VIDEO_STORAGE_ERROR"))?;
        Ok(f(&mut store)?)
    }

    pub async fn run(
        &self,
        persona: String,
        lease: Lease,
        work: VideoWork,
        cancel: CancellationToken,
    ) {
        let heartbeat = async {
            let mut timer = tokio::time::interval(Duration::from_secs(15));
            loop {
                timer.tick().await;
                self.storage(|store| store.heartbeat_task(&lease, self.tick(), 60_000))?;
            }
        };
        let final_result = tokio::select! {
            biased;
            _ = cancel.cancelled() => return,
            failed = heartbeat => failed,
            result = self.process(&persona,&lease,work) => result,
        };
        let saved = self.storage(|store| match final_result {
            Ok(summary) => store.finish_video(&lease, self.tick(), Some(&summary), None),
            Err(error) => store.finish_video(&lease, self.tick(), None, Some(error.0)),
        });
        if saved.is_err() {
            let _ = self.storage(|store| store.expire_leases(self.tick()));
        }
    }

    async fn process(&self, persona: &str, lease: &Lease, work: VideoWork) -> Outcome<String> {
        let decoder = Decoder::installed().await?;
        if decoder.revision != work.models.decoder_revision {
            return Err(Failure("VIDEO_TOOL_CHANGED"));
        }
        let ollama = Ollama::local()?;
        if ollama.inspect(&work.models.visual).await? != work.models.visual_digest {
            return Err(Failure("VIDEO_MODEL_CHANGED"));
        }
        if let Some(model) = &work.models.whisper_path {
            let (fingerprint, _) =
                media::fingerprint(PathBuf::from(model), MAX_WHISPER_BYTES).await?;
            if work.models.whisper_digest.as_ref() != Some(&fingerprint) {
                return Err(Failure("VIDEO_MODEL_CHANGED"));
            }
        }
        let (snapshot, source) = decoder.snapshot(&work.media, &self.root).await?;
        let inspected = decoder.probe(source.clone()).await?;
        if inspected.duration_ms != work.media.duration_ms
            || inspected.has_audio != work.media.has_audio
        {
            return Err(Failure("VIDEO_FILE_CHANGED"));
        }
        for (id, span) in work.pending {
            if let Some(result) =
                self.storage(|store| store.reusable_video_segment(&lease.task_id, &id))?
            {
                self.storage(|store| store.save_video_segment(lease, self.tick(), &id, &result))?;
                continue;
            }
            let directory = tempfile::Builder::new()
                .prefix("segment-")
                .tempdir_in(snapshot.path())
                .map_err(|_| Failure("VIDEO_SCRATCH"))?;
            self.storage(|store| store.video_stage(lease, self.tick(), "decoding"))?;
            let frames = decoder
                .frames(&source, &work.media.container, span, directory.path())
                .await;
            self.storage(|store| store.video_stage(lease, self.tick(), "analyzing"))?;
            let mut result = SegmentAnalysis::default();
            if let Some(model) = work.models.whisper_path.as_deref() {
                match decoder
                    .audio(
                        &source,
                        &work.media.container,
                        span,
                        directory.path(),
                        Path::new(model),
                    )
                    .await
                {
                    Ok(audio) => {
                        result.audio_ms = audio.processed_ms;
                        result.speech = audio.speech;
                    }
                    Err(error) => {
                        result.error = Some(error.0.into());
                    }
                }
            }
            match frames {
                Ok(frames) => {
                    result.samples_ms = frames.times_ms.clone();
                    match ollama
                        .segment(&work.models.visual, span, &frames, &result.speech)
                        .await
                    {
                        Ok(result_model) => {
                            result.summary = Some(result_model.summary);
                            result.events = result_model.events;
                            result.uncertainty = result_model.uncertainty;
                        }
                        Err(error) => {
                            result.error = Some(error.0.into());
                        }
                    }
                }
                Err(error) => {
                    result.error = Some(error.0.into());
                }
            }
            self.storage(|store| store.save_video_segment(lease, self.tick(), &id, &result))?;
        }
        let parts = self.storage(|store| store.video_parts(persona, &lease.task_id))?;
        if !parts.iter().any(|part| {
            part.evidence
                .as_ref()
                .is_some_and(|result| result.summary.is_some())
        }) {
            return Err(Failure("VIDEO_NO_ANALYSIS"));
        }
        self.storage(|store| store.video_stage(lease, self.tick(), "organizing"))?;
        let notes = parts
            .iter()
            .map(|part| {
                let evidence = part.evidence.as_ref();
                let summary = evidence
                    .and_then(|value| value.summary.as_deref())
                    .unwrap_or("画面分析缺失");
                let uncertainty = evidence
                    .and_then(|value| value.uncertainty.as_deref())
                    .map(|s| excerpt(s, 44));
                format!(
                    "{}–{}ms {}{} {}",
                    part.interval.start_ms,
                    part.interval.end_ms,
                    if part.state == "completed" {
                        ""
                    } else {
                        "有覆盖缺口 "
                    },
                    excerpt(summary, 125),
                    uncertainty.unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let summary = ollama.summarize(&work.models.visual, &notes).await?;
        if ollama.inspect(&work.models.visual).await? != work.models.visual_digest {
            return Err(Failure("VIDEO_MODEL_CHANGED"));
        }
        Ok(summary)
    }
}

#[cfg(test)]
mod tests;
