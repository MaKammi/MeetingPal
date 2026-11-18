use meetingpal_lib::{
    capture,
    domain::*,
    exports,
    media::{self, Runtimes},
    storage::Store,
};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64},
        Arc, Mutex,
    },
    time::Duration,
};

/// Run explicitly after building runtimes. No microphone, screen or cloud is used.
#[tokio::test]
#[ignore = "requires all bundled runtimes"]
async fn video_import_playback_visual_evidence_and_exports() {
    let temp = tempfile::tempdir().unwrap();
    let runtimes = Runtimes {
        directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
    };
    for (name, argument) in [("whisper-cli", "--help"), ("llama-server", "--version")] {
        media::run(
            &runtimes.get(name).unwrap(),
            &[argument.into()],
            Duration::from_secs(30),
        )
        .await
        .unwrap();
    }
    let store = Store::open(temp.path().join("library")).unwrap();
    // Exercise the actual recording encoder with RGBA input, including its alpha channel.
    // A synthetic YUV video alone would miss VP8's auto_alt_ref/transparency incompatibility.
    let raw_folder = temp.path().join("raw-capture");
    std::fs::create_dir_all(&raw_folder).unwrap();
    let cropped = capture::crop_frame(
        image::RgbaImage::from_pixel(160, 90, image::Rgba([30, 90, 200, 255])),
        Some(&Crop {
            x: 10,
            y: 10,
            width: 80,
            height: 50,
        }),
    )
    .unwrap();
    capture::encode_video(
        runtimes.get("ffmpeg").unwrap(),
        raw_folder.clone(),
        Arc::new(Mutex::new(Some(cropped))),
        Arc::new(AtomicU64::new(12_000)),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let encoded: Vec<_> = std::fs::read_dir(&raw_folder)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().is_some_and(|s| s == "webm"))
        .collect();
    assert_eq!(encoded.len(), 3);
    let raw_probe = media::run(
        &runtimes.get("ffprobe").unwrap(),
        &[
            "-v".into(),
            "error".into(),
            "-show_streams".into(),
            "-of".into(),
            "json".into(),
            raw_folder
                .join("video-000000.webm")
                .to_string_lossy()
                .into(),
        ],
        Duration::from_secs(30),
    )
    .await
    .unwrap();
    let raw_probe: serde_json::Value = serde_json::from_slice(&raw_probe.stdout).unwrap();
    assert_eq!(raw_probe["streams"][0]["width"], 80);
    assert_eq!(raw_probe["streams"][0]["height"], 50);
    assert_eq!(raw_probe["streams"][0]["pix_fmt"], "yuv420p");
    let wav = temp.path().join("fixture.wav");
    media::create_synthetic_audio(&wav, 12).unwrap();
    let video = temp.path().join("fixture.webm");
    let args = [
        "-hide_banner",
        "-loglevel",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        "testsrc2=size=160x90:rate=10",
        "-i",
        wav.to_str().unwrap(),
        "-t",
        "12",
        "-c:v",
        "libvpx",
        "-c:a",
        "libopus",
        video.to_str().unwrap(),
    ]
    .map(str::to_string);
    media::run(
        &runtimes.get("ffmpeg").unwrap(),
        &args,
        Duration::from_secs(60),
    )
    .await
    .unwrap();
    let config = MeetingConfig {
        title: "Test äöü".into(),
        consent: true,
        ..Default::default()
    };
    let meeting = media::import(&store, &runtimes, &video, config)
        .await
        .unwrap();
    assert!(meeting.duration_ms.abs_diff(12000) < 100);
    assert_eq!(meeting.media.len(), 3);
    assert!(meeting
        .media
        .iter()
        .all(|s| s.microphone.is_none() && s.desktop.is_none()));
    let playback = media::playback(&store, &runtimes, &meeting.id)
        .await
        .unwrap();
    assert_eq!(playback, "playback.webm");
    let frames = media::extract_frames(&store, &runtimes, &meeting)
        .await
        .unwrap();
    assert!(!frames.is_empty());
    assert!(frames[0].1 < 100);
    let clips = media::native_clips(&store, &runtimes, &meeting)
        .await
        .unwrap();
    assert_eq!(clips.len(), 1);
    assert!(clips[0].1 < 100);
    let mut meeting = store.load(&meeting.id).unwrap();
    meeting.transcript.push(TranscriptSegment {
        id: "sample".into(),
        start_ms: 0,
        end_ms: 5000,
        text: "Prüfung bestanden — meeting evidence.".into(),
        speaker: Some("A".into()),
        provisional: false,
        edited: false,
    });
    for format in ["md", "pdf", "json", "srt", "vtt"] {
        let file = temp.path().join(format!("export.{format}"));
        exports::export(&meeting, &file, format).unwrap();
        assert!(std::fs::metadata(file).unwrap().len() > 20);
    }
    let clipped = image::RgbaImage::from_pixel(160, 90, image::Rgba([1, 2, 3, 255]));
    assert_eq!(
        capture::crop_frame(
            clipped,
            Some(&Crop {
                x: 10,
                y: 10,
                width: 80,
                height: 50
            })
        )
        .unwrap()
        .dimensions(),
        (80, 50)
    );
}
