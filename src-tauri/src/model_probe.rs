use crate::{
    domain::*,
    media::{self, Runtimes},
    models::Models,
    providers::{Ai, AnalysisProvider, VisionProvider},
    secrets::Secrets,
    storage::Store,
    AppState,
};
use anyhow::{ensure, Result};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Task {
    Audio,
    Text,
    Image,
    Video,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub model: String,
    pub task: Task,
    pub elapsed_ms: u64,
    pub requests: u32,
}

/// Exercise the same adapters as a real meeting with an isolated library and fixed public inputs.
pub async fn run(
    secrets: Secrets,
    runtimes: Runtimes,
    selection: ModelSelection,
    task: Task,
) -> Result<Report> {
    ensure!(
        selection.provider != Provider::Local
            && !selection.model.trim().is_empty()
            && selection.model.len() <= 200,
        "Choose a cloud model for this test"
    );
    let temp = tempfile::tempdir()?;
    let state = AppState {
        store: Store::open(temp.path().into())?,
        models: Models::new(temp.path().join("models")),
        secrets,
        runtimes,
        ai: Ai::new()?,
        recording: Default::default(),
        updates: Default::default(),
    };
    let mut config = MeetingConfig {
        title: "MeetingPal API test".into(),
        language: "en".into(),
        consent: true,
        offline: false,
        max_requests: 40,
        ..Default::default()
    };
    match task {
        Task::Audio => {
            ensure!(
                selection.provider == Provider::Openai,
                "Audio transcription requires OpenAI"
            );
            config.transcription = selection.clone();
            config.stream_transcription = true;
        }
        Task::Text => config.analysis = selection.clone(),
        Task::Image | Task::Video => {
            config.video.kind = VideoKind::Portal;
            config.vision = selection.clone();
            config.video_analysis = if matches!(task, Task::Video) {
                VideoAnalysis::Native
            } else {
                VideoAnalysis::Frames
            };
        }
    }
    config.validate()?;
    let mut meeting = Meeting::new(config);
    meeting.status = "recording".into();
    state.store.save(&meeting)?;
    let folder = state.store.folder(&meeting.id)?;
    let started = Instant::now();
    match task {
        Task::Audio => {
            let wav = include_bytes!("../../resources/benchmark.wav");
            std::fs::write(folder.join("public.wav"), wav)?;
            let reader = hound::WavReader::new(&wav[..])?;
            let segment = MediaSegment {
                id: "fixture".into(),
                start_ms: 0,
                end_ms: u64::from(reader.duration()) * 1000 / u64::from(reader.spec().sample_rate),
                microphone: None,
                desktop: None,
                mixed: "public.wav".into(),
            };
            let received = std::sync::atomic::AtomicBool::new(false);
            let result = state
                .ai
                .transcribe_with_progress(&state, &meeting, &segment, |text| {
                    if !text.is_empty() {
                        received.store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                })
                .await?;
            ensure!(
                result
                    .iter()
                    .any(|s| s.text.to_lowercase().contains("country")),
                "The model did not recognize the public test speech"
            );
            if crate::transcription::model_supports("openai", &selection.model, "audioStream") {
                ensure!(
                    received.load(std::sync::atomic::Ordering::Relaxed),
                    "No streaming transcript received"
                );
            }
        }
        Task::Text => {
            meeting.transcript.push(TranscriptSegment {
                id: "sample".into(),
                start_ms: 0,
                end_ms: 5000,
                text: "The team decided to hold the next meeting on Tuesday.".into(),
                speaker: None,
                provisional: false,
                edited: false,
            });
            state.store.save(&meeting)?;
            let analysis = state.ai.analyze(&state, &meeting, false).await?;
            analysis.validate_sources(&meeting.source_ids())?;
            ensure!(
                !analysis.summary.is_empty(),
                "Missing summary for the test transcript"
            );
        }
        Task::Image | Task::Video => {
            let mut image = image::RgbImage::from_pixel(160, 90, image::Rgb([240u8, 240, 240]));
            for x in 30..110 {
                for y in 20..65 {
                    image.put_pixel(x, y, image::Rgb([15, 90, 200]));
                }
            }
            image.save(folder.join("slide.jpg"))?;
            let video = matches!(task, Task::Video);
            let name = if video {
                let input = folder.join("slide.jpg");
                let output = folder.join("slide.webm");
                let input = input.to_string_lossy();
                let output = output.to_string_lossy();
                let args = [
                    "-y",
                    "-loop",
                    "1",
                    "-i",
                    input.as_ref(),
                    "-t",
                    "2",
                    "-r",
                    "2",
                    "-c:v",
                    "libvpx",
                    "-pix_fmt",
                    "yuv420p",
                    output.as_ref(),
                ]
                .map(String::from);
                media::run(
                    &state.runtimes.get("ffmpeg")?,
                    &args,
                    Duration::from_secs(30),
                )
                .await?;
                "slide.webm"
            } else {
                "slide.jpg"
            };
            let description = state.ai.describe(&state, &meeting, name, 0, video).await?;
            ensure!(
                !description.trim().is_empty(),
                "The provider returned an empty visual description"
            );
            ensure!(
                state.store.load(&meeting.id)?.warnings.is_empty(),
                "Provider file cleanup did not complete"
            );
        }
    }
    Ok(Report {
        model: selection.model,
        task,
        elapsed_ms: started.elapsed().as_millis() as u64,
        requests: state.store.load(&meeting.id)?.cloud_requests,
    })
}
