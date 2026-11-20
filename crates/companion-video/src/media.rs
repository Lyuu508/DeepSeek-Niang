use crate::{Failure, Outcome};
use companion_core::{
    companion::excerpt,
    video::{Interval, LocalMedia, MAX_VIDEO_BYTES, MAX_VIDEO_MS, SpeechCue},
};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

const SHORT: Duration = Duration::from_secs(30);
const LONG: Duration = Duration::from_secs(210);

pub struct Decoder {
    ffmpeg: PathBuf,
    ffprobe: PathBuf,
    pub revision: String,
}

pub struct Frames {
    pub times_ms: Vec<u64>,
    pub jpeg: Vec<Vec<u8>>,
}

pub struct Audio {
    pub processed_ms: u64,
    pub speech: Vec<SpeechCue>,
}

pub fn local_file(path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    #[cfg(windows)]
    if !matches!(path.components().next(), Some(std::path::Component::Prefix(part))
        if matches!(part.kind(), std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)))
    {
        return false;
    }
    path.is_file()
}

fn binary(name: &str) -> Outcome<PathBuf> {
    let file = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .map(|dir| dir.join(&file))
        .find(|path| local_file(path))
        .ok_or(Failure(if name.starts_with("ff") {
            "VIDEO_FFMPEG_MISSING"
        } else {
            "VIDEO_WHISPER_MISSING"
        }))
}

fn command(path: &Path) -> Command {
    let mut command = Command::new(path);
    command
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    command
}

async fn output(
    mut command: Command,
    max: usize,
    deadline: Duration,
    code: &'static str,
) -> Outcome<Vec<u8>> {
    let mut child = command.spawn().map_err(|_| Failure(code))?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or(Failure(code))?
        .take((max + 1) as u64);
    timeout(deadline, async {
        let mut bytes = Vec::new();
        stdout
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| Failure(code))?;
        if bytes.len() > max || !child.wait().await.map_err(|_| Failure(code))?.success() {
            return Err(Failure(code));
        }
        Ok(bytes)
    })
    .await
    .map_err(|_| Failure("VIDEO_TOOL_TIMEOUT"))?
}

pub fn file_digest(path: &Path, limit: u64, mut copy: Option<&mut File>) -> Outcome<(String, u64)> {
    if !local_file(path) {
        return Err(Failure("VIDEO_FILE_UNAVAILABLE"));
    }
    let mut file = File::open(path).map_err(|_| Failure("VIDEO_FILE_UNAVAILABLE"))?;
    let bytes = file
        .metadata()
        .map_err(|_| Failure("VIDEO_FILE_UNAVAILABLE"))?
        .len();
    if bytes == 0 || bytes > limit {
        return Err(Failure("VIDEO_FILE_SIZE"));
    }
    let mut hash = Sha256::new();
    let mut data = vec![0; 128 * 1024];
    let mut actual = 0;
    loop {
        let n = file
            .read(&mut data)
            .map_err(|_| Failure("VIDEO_FILE_UNAVAILABLE"))?;
        if n == 0 {
            break;
        }
        actual += n as u64;
        if actual > limit || actual > bytes {
            return Err(Failure("VIDEO_FILE_CHANGED"));
        }
        hash.update(&data[..n]);
        if let Some(target) = copy.as_mut() {
            target
                .write_all(&data[..n])
                .map_err(|_| Failure("VIDEO_SCRATCH"))?;
        }
    }
    if actual != bytes {
        return Err(Failure("VIDEO_FILE_CHANGED"));
    }
    let digest = hash
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    Ok((digest, bytes))
}

pub async fn fingerprint(path: PathBuf, limit: u64) -> Outcome<(String, u64)> {
    tokio::task::spawn_blocking(move || file_digest(&path, limit, None))
        .await
        .map_err(|_| Failure("VIDEO_FILE_UNAVAILABLE"))?
}

fn container(path: &Path) -> Outcome<&'static str> {
    match path
        .extension()
        .and_then(OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("mp4" | "mov" | "m4v") => Ok("mov"),
        Some("mkv" | "webm") => Ok("matroska"),
        _ => Err(Failure("VIDEO_FILE_FORMAT")),
    }
}

fn input(command: &mut Command, format: &str, path: &Path) {
    command.args([
        "-protocol_whitelist",
        "file,pipe",
        "-format_whitelist",
        "mov,matroska",
        "-max_alloc",
        "67108864",
        "-f",
        format,
        "-err_detect",
        "explode",
    ]);
    if format == "mov" {
        command.args(["-enable_drefs", "0", "-use_absolute_path", "0"]);
    }
    command.arg("-i").arg(path);
}

fn seconds(ms: u64) -> String {
    format!("{}.{:03}", ms / 1000, ms % 1000)
}

impl Decoder {
    pub async fn installed() -> Outcome<Self> {
        let ffmpeg = binary("ffmpeg")?;
        let ffprobe = binary("ffprobe")?;
        let mut revisions = Vec::new();
        for (name, path) in [("ffmpeg", &ffmpeg), ("ffprobe", &ffprobe)] {
            let mut check = command(path);
            check.arg("-version");
            let bytes = output(
                check,
                16 * 1024,
                Duration::from_secs(5),
                "VIDEO_FFMPEG_MISSING",
            )
            .await?;
            let version = String::from_utf8_lossy(&bytes);
            let line = version.lines().next().unwrap_or_default();
            if !line.starts_with(&format!("{name} version ")) {
                return Err(Failure("VIDEO_FFMPEG_MISSING"));
            }
            revisions.push(excerpt(line, 110));
        }
        Ok(Self {
            ffmpeg,
            ffprobe,
            revision: revisions.join("|"),
        })
    }

    pub async fn probe(&self, path: PathBuf) -> Outcome<LocalMedia> {
        let format = container(&path)?;
        let (checksum, bytes) = fingerprint(path.clone(), MAX_VIDEO_BYTES).await?;
        let mut query = command(&self.ffprobe);
        query.args(["-v","error","-probesize","16777216","-analyzeduration","10000000",
            "-show_entries","format=duration:stream=codec_type,width,height,duration:stream_disposition=attached_pic","-of","json"]);
        input(&mut query, format, &path);
        let body = output(query, 100 * 1024, SHORT, "VIDEO_PROBE_FAILED").await?;
        let info: Value =
            serde_json::from_slice(&body).map_err(|_| Failure("VIDEO_PROBE_FAILED"))?;
        let tracks = info["streams"]
            .as_array()
            .ok_or(Failure("VIDEO_PROBE_FAILED"))?;
        let visual = tracks
            .iter()
            .find(|stream| stream["codec_type"] == "video")
            .ok_or(Failure("VIDEO_NO_VIDEO_TRACK"))?;
        let w = visual["width"].as_u64().unwrap_or(0);
        let h = visual["height"].as_u64().unwrap_or(0);
        if visual["disposition"]["attached_pic"] == 1 || w == 0 || h == 0 || w > 4096 || h > 4096 {
            return Err(Failure("VIDEO_DIMENSIONS"));
        }
        let duration = visual["duration"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .or_else(|| {
                info["format"]["duration"]
                    .as_str()
                    .and_then(|s| s.parse().ok())
            })
            .filter(|value| value.is_finite() && *value > 0.0)
            .ok_or(Failure("VIDEO_PROBE_FAILED"))?;
        let duration_ms = (duration * 1000.0).round() as u64;
        if !(1..=MAX_VIDEO_MS).contains(&duration_ms) {
            return Err(Failure("VIDEO_DURATION"));
        }
        let title = path
            .file_name()
            .and_then(OsStr::to_str)
            .ok_or(Failure("VIDEO_FILE_UNAVAILABLE"))?;
        let title = excerpt(title, 140);
        let name = path
            .into_os_string()
            .into_string()
            .map_err(|_| Failure("VIDEO_FILE_UNAVAILABLE"))?;
        Ok(LocalMedia {
            path: name,
            title,
            checksum,
            container: format.into(),
            bytes,
            duration_ms,
            has_audio: tracks.iter().any(|stream| stream["codec_type"] == "audio"),
        })
    }

    pub async fn snapshot(
        &self,
        media: &LocalMedia,
        parent: &Path,
    ) -> Outcome<(tempfile::TempDir, PathBuf)> {
        let parent = parent.to_owned();
        let media = media.clone();
        tokio::task::spawn_blocking(move || {
            fs::create_dir_all(&parent).map_err(|_| Failure("VIDEO_SCRATCH"))?;
            let folder = tempfile::Builder::new()
                .prefix("analysis-")
                .tempdir_in(&parent)
                .map_err(|_| Failure("VIDEO_SCRATCH"))?;
            let target = folder.path().join(if media.container == "mov" {
                "input.mp4"
            } else {
                "input.mkv"
            });
            let mut copy = File::create_new(&target).map_err(|_| Failure("VIDEO_SCRATCH"))?;
            let (checksum, length) =
                file_digest(Path::new(&media.path), MAX_VIDEO_BYTES, Some(&mut copy))?;
            if checksum != media.checksum || length != media.bytes {
                return Err(Failure("VIDEO_FILE_CHANGED"));
            }
            drop(copy);
            Ok((folder, target))
        })
        .await
        .map_err(|_| Failure("VIDEO_SCRATCH"))?
    }

    fn decode(&self, source: &Path, format: &str, span: Interval) -> Command {
        let mut command = command(&self.ffmpeg);
        command
            .args([
                "-hide_banner",
                "-nostdin",
                "-loglevel",
                "error",
                "-xerror",
                "-threads",
                "2",
                "-filter_threads",
                "2",
                "-ss",
            ])
            .arg(seconds(span.start_ms));
        input(&mut command, format, source);
        command.arg("-t").arg(seconds(span.end_ms - span.start_ms));
        command
    }

    pub async fn frames(
        &self,
        source: &Path,
        format: &str,
        span: Interval,
        folder: &Path,
    ) -> Outcome<Frames> {
        let mut process = self.decode(source, format, span);
        // Regular 1 Hz samples, with an extra sample on visual changes from a 2 Hz stream.
        process.args(["-map","0:v:0","-an","-sn","-dn","-vf",
            "fps=2:start_time=0:round=up:eof_action=pass,scale=384:384:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1,select='not(mod(n,2))+gt(scene,0.18)'",
            "-c:v","mjpeg","-q:v","5","-fps_mode","passthrough","-enc_time_base","1:2",
            "-frames:v","17","-f","image2","-frame_pts","1"]).arg(folder.join("frame-%d.jpg"));
        output(process, 4096, SHORT, "VIDEO_DECODE_FAILED").await?;
        let mut frames = Frames {
            times_ms: Vec::new(),
            jpeg: Vec::new(),
        };
        for sample in 0..17 {
            let at = span.start_ms + sample * 500;
            if at >= span.end_ms {
                break;
            }
            let path = folder.join(format!("frame-{sample}.jpg"));
            if !path.is_file() {
                continue;
            }
            let data = read_limited(&path, 220 * 1024).await?;
            if !data.starts_with(b"\xff\xd8") {
                return Err(Failure("VIDEO_DECODE_FAILED"));
            }
            frames.times_ms.push(at);
            frames.jpeg.push(data);
        }
        if frames.jpeg.is_empty() {
            return Err(Failure("VIDEO_DECODE_FAILED"));
        }
        Ok(frames)
    }

    pub async fn audio(
        &self,
        source: &Path,
        format: &str,
        span: Interval,
        folder: &Path,
        model: &Path,
    ) -> Outcome<Audio> {
        let duration = span.end_ms - span.start_ms;
        let pcm = self.pcm(source, format, span).await?;
        if pcm.len() < 32 || pcm.len() % 2 != 0 {
            return Err(Failure("VIDEO_AUDIO_DECODE_FAILED"));
        }
        let processed_ms = (pcm.len() as u64 / 32).min(duration);
        let wav = folder.join("speech.wav");
        let result = folder.join("speech");
        tokio::fs::write(&wav, wave(&pcm))
            .await
            .map_err(|_| Failure("VIDEO_SCRATCH"))?;
        let mut whisper = command(&binary("whisper-cli")?);
        whisper
            .arg("-m")
            .arg(model)
            .arg("-f")
            .arg(wav)
            .args(["-t", "2", "-l", "auto", "-ng", "-np", "-oj", "-of"])
            .arg(&result);
        output(whisper, 16 * 1024, LONG, "VIDEO_TRANSCRIBE_FAILED").await?;
        let body = read_limited(&result.with_extension("json"), 256 * 1024).await?;
        Ok(Audio {
            processed_ms,
            speech: parse_speech(&body, span.start_ms, processed_ms)?,
        })
    }

    pub(crate) async fn pcm(
        &self,
        source: &Path,
        format: &str,
        span: Interval,
    ) -> Outcome<Vec<u8>> {
        let mut process = self.decode(source, format, span);
        process.args([
            "-map",
            "0:a:0",
            "-vn",
            "-sn",
            "-dn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-c:a",
            "pcm_s16le",
            "-f",
            "s16le",
            "pipe:1",
        ]);
        output(
            process,
            ((span.end_ms - span.start_ms + 125) * 32) as usize,
            SHORT,
            "VIDEO_AUDIO_DECODE_FAILED",
        )
        .await
    }
}

pub fn check_whisper() -> Outcome<()> {
    binary("whisper-cli").map(drop)
}

async fn read_limited(path: &Path, cap: usize) -> Outcome<Vec<u8>> {
    let file = tokio::fs::File::open(path)
        .await
        .map_err(|_| Failure("VIDEO_READ_RESULT"))?;
    let mut result = Vec::new();
    file.take((cap + 1) as u64)
        .read_to_end(&mut result)
        .await
        .map_err(|_| Failure("VIDEO_READ_RESULT"))?;
    if result.len() > cap {
        return Err(Failure("VIDEO_READ_RESULT"));
    }
    Ok(result)
}

fn wave(pcm: &[u8]) -> Vec<u8> {
    let length = pcm.len() as u32;
    let mut wave = Vec::with_capacity(pcm.len() + 44);
    wave.extend(b"RIFF");
    wave.extend((length + 36).to_le_bytes());
    wave.extend(b"WAVEfmt \x10\0\0\0\x01\0\x01\0");
    wave.extend(16000_u32.to_le_bytes());
    wave.extend(32000_u32.to_le_bytes());
    wave.extend(b"\x02\0\x10\0data");
    wave.extend(length.to_le_bytes());
    wave.extend(pcm);
    wave
}

#[derive(Deserialize)]
struct Transcript {
    transcription: Vec<Utterance>,
}
#[derive(Deserialize)]
struct Utterance {
    offsets: Offsets,
    text: String,
}
#[derive(Deserialize)]
struct Offsets {
    from: u64,
    to: u64,
}
pub(crate) fn parse_speech(json: &[u8], start: u64, duration: u64) -> Outcome<Vec<SpeechCue>> {
    let data: Transcript =
        serde_json::from_slice(json).map_err(|_| Failure("VIDEO_TRANSCRIBE_FAILED"))?;
    let mut cues = Vec::new();
    for item in data.transcription.into_iter().take(40) {
        let from = item.offsets.from.min(duration);
        let to = item.offsets.to.min(duration);
        if from < to && !item.text.trim().is_empty() {
            cues.push(SpeechCue {
                start_ms: start + from,
                end_ms: start + to,
                text: excerpt(item.text.trim(), 400),
            });
        }
    }
    Ok(cues)
}
