use crate::{
    capture::{hidden_command, write_wav},
    domain::*,
    storage::{safe_child, Store},
};
use anyhow::{Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone)]
pub struct Runtimes {
    pub directory: PathBuf,
}
impl Runtimes {
    pub fn get(&self, name: &str) -> Result<PathBuf> {
        let file = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        };
        let candidate = self.directory.join(&file);
        anyhow::ensure!(candidate.is_file(), "The bundled {name} runtime is missing. Install a complete MeetingPal package, or run npm run runtimes:build in a source checkout.");
        Ok(candidate)
    }
    pub fn status(&self) -> std::collections::HashMap<String, bool> {
        ["ffmpeg", "ffprobe", "whisper-cli", "llama-server"]
            .iter()
            .map(|n| (n.to_string(), self.get(n).is_ok()))
            .collect()
    }
}
pub async fn run(
    program: &Path,
    args: &[String],
    timeout: Duration,
) -> Result<std::process::Output> {
    let mut command: tokio::process::Command = hidden_command(program).into();
    command.args(args).kill_on_drop(true);
    let output = tokio::time::timeout(timeout, command.output())
        .await
        .context("Media operation timed out")??;
    anyhow::ensure!(
        output.status.success(),
        "Media process failed ({}). Check the source format and available disk space.",
        program.file_stem().unwrap_or_default().to_string_lossy()
    );
    Ok(output)
}
pub fn read_wav(path: &Path) -> Result<(Vec<f32>, u32)> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    anyhow::ensure!(
        spec.channels == 1
            && spec.sample_format == hound::SampleFormat::Int
            && spec.bits_per_sample == 16,
        "Expected mono PCM16 WAV."
    );
    let samples = reader
        .samples::<i16>()
        .map(|s| s.map(|v| v as f32 / 32768.0))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((samples, spec.sample_rate))
}
pub fn concat_wav(folder: &Path, segments: &[MediaSegment], output: &Path) -> Result<()> {
    let mut writer = hound::WavWriter::create(
        output,
        hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )?;
    for segment in segments {
        let mut reader = hound::WavReader::open(safe_child(folder, &segment.mixed)?)?;
        anyhow::ensure!(
            reader.spec().sample_rate == 16000 && reader.spec().channels == 1,
            "Invalid audio segment format"
        );
        for sample in reader.samples::<i16>() {
            writer.write_sample(sample?)?;
        }
    }
    writer.finalize()?;
    Ok(())
}
fn concat_list(folder: &Path, files: &[String]) -> Result<PathBuf> {
    let path = folder.join("video-concat.txt");
    let mut contents = String::new();
    for file in files {
        safe_child(folder, file)?;
        anyhow::ensure!(!file.contains(['\'', '\n', '\r']), "Invalid media filename");
        contents.push_str(&format!("file '{file}'\n"));
    }
    fs::write(&path, contents)?;
    Ok(path)
}
pub async fn playback(store: &Store, runtimes: &Runtimes, id: &str) -> Result<String> {
    let meeting = store.load(id)?;
    let folder = store.folder(id)?;
    anyhow::ensure!(
        !meeting.media.is_empty(),
        "This meeting has no audio files."
    );
    concat_wav(&folder, &meeting.media, &folder.join("playback.wav"))?;
    let file = if meeting.video_files.is_empty() {
        "playback.wav"
    } else {
        let list = concat_list(&folder, &meeting.video_files)?;
        let args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-f".into(),
            "concat".into(),
            "-safe".into(),
            "1".into(),
            "-i".into(),
            list.to_string_lossy().into(),
            "-i".into(),
            folder.join("playback.wav").to_string_lossy().into(),
            "-map".into(),
            "0:v:0".into(),
            "-map".into(),
            "1:a:0".into(),
            "-c:v".into(),
            "copy".into(),
            "-c:a".into(),
            "libopus".into(),
            folder.join("playback.webm").to_string_lossy().into(),
        ];
        run(&runtimes.get("ffmpeg")?, &args, Duration::from_secs(600)).await?;
        "playback.webm"
    };
    store.update(id, |m| {
        m.playback_file = Some(file.into());
        Ok(())
    })?;
    Ok(file.into())
}
pub async fn import(
    store: &Store,
    runtimes: &Runtimes,
    path: &Path,
    mut config: MeetingConfig,
) -> Result<Meeting> {
    anyhow::ensure!(path.is_file(), "Select an existing audio or video file.");
    let extension = path
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    anyhow::ensure!(
        ["wav", "mp3", "m4a", "flac", "ogg", "opus", "webm", "mp4", "mkv", "mov", "aac"]
            .contains(&extension.as_str()),
        "Unsupported media extension."
    );
    config.validate()?;
    config.mode = Mode::Post;
    let mut meeting = Meeting::new(config);
    meeting.status = "importing".into();
    store.save(&meeting)?;
    let folder = store.folder(&meeting.id)?;
    let result: Result<()> = async {
        let probe = run(
            &runtimes.get("ffprobe")?,
            &[
                "-v".into(),
                "error".into(),
                "-show_streams".into(),
                "-show_format".into(),
                "-of".into(),
                "json".into(),
                path.to_string_lossy().into(),
            ],
            Duration::from_secs(30),
        )
        .await?;
        let metadata: serde_json::Value = serde_json::from_slice(&probe.stdout)?;
        let streams = metadata["streams"]
            .as_array()
            .context("No media streams found")?;
        anyhow::ensure!(
            streams.iter().any(|s| s["codec_type"] == "audio"),
            "The imported file has no audio track."
        );
        let has_video = streams.iter().any(|s| s["codec_type"] == "video");
        let original = format!("original.{extension}");
        fs::copy(path, folder.join(&original))?;
        run(
            &runtimes.get("ffmpeg")?,
            &[
                "-hide_banner".into(),
                "-loglevel".into(),
                "error".into(),
                "-y".into(),
                "-i".into(),
                folder.join(&original).to_string_lossy().into(),
                "-vn".into(),
                "-ac".into(),
                "1".into(),
                "-ar".into(),
                "16000".into(),
                "-c:a".into(),
                "pcm_s16le".into(),
                "-f".into(),
                "segment".into(),
                "-segment_time".into(),
                "5".into(),
                folder.join("mixed-%06d.wav").to_string_lossy().into(),
            ],
            Duration::from_secs(1800),
        )
        .await?;
        let mut files: Vec<_> = fs::read_dir(&folder)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("mixed-")
            })
            .collect();
        files.sort();
        let mut start = 0;
        for (index, file) in files.iter().enumerate() {
            let reader = hound::WavReader::open(file)?;
            let duration = reader.duration() as u64 * 1000 / 16000;
            meeting.media.push(MediaSegment {
                id: format!("audio-{index:06}"),
                start_ms: start,
                end_ms: start + duration,
                microphone: None,
                desktop: None,
                mixed: file.file_name().unwrap().to_string_lossy().into(),
            });
            start += duration;
        }
        meeting.duration_ms = start;
        if has_video {
            run(
                &runtimes.get("ffmpeg")?,
                &[
                    "-hide_banner".into(),
                    "-loglevel".into(),
                    "error".into(),
                    "-y".into(),
                    "-i".into(),
                    folder.join(&original).to_string_lossy().into(),
                    "-an".into(),
                    "-vf".into(),
                    "scale='min(1920,iw)':-2,fps=10".into(),
                    "-c:v".into(),
                    "libvpx".into(),
                    "-deadline".into(),
                    "realtime".into(),
                    "-b:v".into(),
                    "1500k".into(),
                    "-g".into(),
                    "50".into(),
                    folder.join("video-import.webm").to_string_lossy().into(),
                ],
                Duration::from_secs(7200),
            )
            .await?;
            meeting.video_files.push("video-import.webm".into());
        } else {
            meeting.config.video_analysis = VideoAnalysis::None;
        }
        meeting.status = "recorded".into();
        store.save(&meeting)?;
        Ok(())
    }
    .await;
    if let Err(error) = result {
        store.update(&meeting.id, |m| {
            m.status = "failed".into();
            m.warnings.push(error.to_string());
            Ok(())
        })?;
        return Err(error);
    }
    Ok(meeting)
}

pub async fn extract_frames(
    store: &Store,
    runtimes: &Runtimes,
    meeting: &Meeting,
) -> Result<Vec<(String, u64)>> {
    let folder = store.folder(&meeting.id)?;
    if meeting.video_files.is_empty() {
        return Ok(vec![]);
    }
    let list = concat_list(&folder, &meeting.video_files)?;
    let args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "info".into(),
        "-y".into(),
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "1".into(),
        "-i".into(),
        list.to_string_lossy().into(),
        "-vf".into(),
        "select='eq(n,0)+gt(scene,0.18)*gte(t-prev_selected_t,5)',scale='min(1280,iw)':-2,showinfo"
            .into(),
        "-fps_mode".into(),
        "vfr".into(),
        "-frames:v".into(),
        "500".into(),
        "-q:v".into(),
        "3".into(),
        folder.join("frame-%06d.jpg").to_string_lossy().into(),
    ];
    let out = run(&runtimes.get("ffmpeg")?, &args, Duration::from_secs(1800)).await?;
    let mut frames = vec![];
    for line in String::from_utf8_lossy(&out.stderr).lines() {
        if let Some((_, suffix)) = line.split_once("pts_time:") {
            if let Some(seconds) = suffix
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
            {
                let name = format!("frame-{:06}.jpg", frames.len() + 1);
                if folder.join(&name).exists() {
                    frames.push((name, (seconds * 1000.0).max(0.0) as u64));
                }
            }
        }
    }
    Ok(frames)
}
pub async fn native_clips(
    store: &Store,
    runtimes: &Runtimes,
    meeting: &Meeting,
) -> Result<Vec<(String, u64)>> {
    playback(store, runtimes, &meeting.id).await?;
    let folder = store.folder(&meeting.id)?;
    run(
        &runtimes.get("ffmpeg")?,
        &[
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-y".into(),
            "-i".into(),
            folder.join("playback.webm").to_string_lossy().into(),
            "-c".into(),
            "copy".into(),
            "-f".into(),
            "segment".into(),
            "-segment_time".into(),
            "60".into(),
            "-reset_timestamps".into(),
            "1".into(),
            "-segment_list".into(),
            folder.join("clip-index.csv").to_string_lossy().into(),
            folder.join("clip-%06d.webm").to_string_lossy().into(),
        ],
        Duration::from_secs(600),
    )
    .await?;
    let mut clips = Vec::new();
    for line in fs::read_to_string(folder.join("clip-index.csv"))?.lines() {
        let columns = line.split(',').collect::<Vec<_>>();
        anyhow::ensure!(columns.len() == 3, "Invalid video segment index");
        let file = Path::new(columns[0].trim_matches('"'))
            .file_name()
            .context("Missing clip filename")?
            .to_string_lossy()
            .to_string();
        anyhow::ensure!(safe_child(&folder, &file)?.is_file(), "Missing video clip");
        let seconds: f64 = columns[1].parse()?;
        clips.push((file, (seconds.max(0.0) * 1000.0).round() as u64));
    }
    Ok(clips)
}
pub fn silence(samples: &[f32]) -> bool {
    samples.is_empty()
        || samples.iter().map(|s| s * s).sum::<f32>() / (samples.len().max(1) as f32) < 0.0000005
}
pub fn create_synthetic_audio(path: &Path, seconds: u32) -> Result<()> {
    let samples: Vec<_> = (0..seconds * 16000)
        .map(|i| ((i as f32 * 440.0 * std::f32::consts::TAU / 16000.0).sin()) * 0.2)
        .collect();
    write_wav(path, &samples, 16000)
}
