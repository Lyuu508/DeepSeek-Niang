use super::*;
use companion_core::video::{Interval, plan_segments};
use serde_json::json;
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    process::Command,
};

#[test]
fn transcript_timestamps_are_clamped_to_decoded_audio_and_rebased() {
    let doc = json!({"transcription":[
        {"offsets":{"from":200,"to":1500},"text":"第一句"},
        {"offsets":{"from":2100,"to":2500},"text":"没有对应音频"}
    ]});
    let result = media::parse_speech(doc.to_string().as_bytes(), 7500, 1100).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!((result[0].start_ms, result[0].end_ms), (7700, 8600));
}

#[tokio::test]
async fn loopback_fixture_checks_model_capability_and_vision_request() {
    let listen = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listen.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for _ in 0..3 {
            let (mut stream, _) = listen.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut start = String::new();
            reader.read_line(&mut start).unwrap();
            let mut length = 0;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap();
                }
            }
            let mut data = vec![0; length];
            reader.read_exact(&mut data).unwrap();
            let response = if start.contains("/api/tags") {
                json!({"models":[{"name":"qwen3-vl:4b","digest":"a".repeat(64)}]})
            } else if start.contains("/api/show") {
                json!({"capabilities":["completion","vision"]})
            } else {
                assert!(start.contains("/api/chat"));
                let request: serde_json::Value = serde_json::from_slice(&data).unwrap();
                assert_eq!(request["model"], "qwen3-vl:4b");
                assert_eq!(request["messages"][1]["images"][0], "/9g=");
                assert_eq!(request["stream"], false);
                json!({"done":true,"message":{"content":json!({"summary":"夹具结果","events":[
                    {"offset_ms":900,"description":"观察"},
                    {"offset_ms":9000,"description":"超出片段"}
                ],"uncertainty":""}).to_string()}})
            }
            .to_string();
            write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",response.len(),response).unwrap();
        }
    });
    let client = Ollama::with_base(url).unwrap();
    assert_eq!(client.inspect("qwen3-vl:4b").await.unwrap(), "a".repeat(64));
    let answer = client
        .segment(
            "qwen3-vl:4b",
            Interval {
                start_ms: 7500,
                end_ms: 9500,
            },
            &media::Frames {
                times_ms: vec![7500],
                jpeg: vec![vec![255, 216]],
            },
            &[],
        )
        .await
        .unwrap();
    assert_eq!(answer.summary, "夹具结果");
    assert_eq!(answer.events.len(), 1);
    assert_eq!(answer.events[0].at_ms, 8400);
    server.join().unwrap();
}

// Explicitly enabled in the Linux media validation job with installed FFmpeg.
#[tokio::test]
async fn real_media_reads_both_halves_and_continuous_audio() {
    if std::env::var_os("COMPANION_VIDEO_MEDIA_TEST").is_none() {
        return;
    }
    let scratch = tempfile::tempdir().unwrap();
    let path = scratch.path().join("授权测试素材.mp4");
    let status = Command::new("ffmpeg")
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:s=160x96:r=10:d=8",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:s=160x96:r=10:d=8",
            "-f",
            "lavfi",
            "-i",
            "sine=f=880:d=16",
            "-filter_complex",
            "[0:v][1:v]concat=n=2:v=1:a=0[v]",
            "-map",
            "[v]",
            "-map",
            "2:a",
            "-c:v",
            "mpeg4",
            "-c:a",
            "aac",
            "-threads",
            "2",
        ])
        .arg(&path)
        .status()
        .unwrap();
    assert!(status.success());
    let decoder = Decoder::installed().await.unwrap();
    let media = decoder.probe(path.clone()).await.unwrap();
    assert_eq!(media.duration_ms, 16_000);
    assert!(media.has_audio);
    let (snapshot, source) = decoder.snapshot(&media, scratch.path()).await.unwrap();
    let plan = plan_segments(media.duration_ms, 8000, 500).unwrap();
    assert_eq!(plan.len(), 3);
    let mut colors = Vec::new();
    for span in plan {
        let section = tempfile::tempdir_in(snapshot.path()).unwrap();
        let frames = decoder
            .frames(&source, &media.container, span, section.path())
            .await
            .unwrap();
        assert_eq!(frames.times_ms[0], span.start_ms);
        assert!(frames.times_ms.last().unwrap() + 1000 >= span.end_ms);
        let frame = section.path().join("test.jpg");
        std::fs::write(&frame, &frames.jpeg[0]).unwrap();
        let sample = Command::new("ffmpeg")
            .args(["-v", "error", "-i"])
            .arg(frame)
            .args([
                "-vf",
                "scale=1:1",
                "-pix_fmt",
                "rgb24",
                "-frames:v",
                "1",
                "-f",
                "rawvideo",
                "pipe:1",
            ])
            .output()
            .unwrap();
        assert!(sample.status.success());
        colors.push(sample.stdout);
        let pcm = decoder.pcm(&source, &media.container, span).await.unwrap();
        assert!(pcm.len() as u64 / 32 >= span.end_ms - span.start_ms - 20);
        assert!(pcm.iter().any(|value| *value != 0));
    }
    assert!(colors.first().unwrap()[0] > 190 && colors.last().unwrap()[2] > 190);
    std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .unwrap()
        .write_all(b"x")
        .unwrap();
    assert_eq!(
        decoder
            .snapshot(&media, scratch.path())
            .await
            .err()
            .unwrap()
            .0,
        "VIDEO_FILE_CHANGED"
    );
}
