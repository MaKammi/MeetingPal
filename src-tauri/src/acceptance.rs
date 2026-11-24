use crate::{domain::*, exports, jobs, media, providers::AnalysisProvider, AppState};
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::time::Instant;

pub(crate) struct Startup(pub Option<std::sync::Arc<tokio::sync::Notify>>);

/// Exercise installed runtimes and persistence using the public fixture in a temporary library.
pub async fn offline(state: &AppState, with_models: bool, startup_ms: u128) -> Result<Value> {
    let started = Instant::now();
    ensure!(
        state.runtimes.status().values().all(|v| *v),
        "The installed runtime package is incomplete"
    );
    let wav = state.store.root.join("public.wav");
    std::fs::write(&wav, include_bytes!("../../resources/benchmark.wav"))?;
    let config = MeetingConfig {
        title: "Offline acceptance".into(),
        consent: true,
        language: "en".into(),
        ..Default::default()
    };
    eprintln!("Installed check: importing public fixture");
    let meeting = media::import(&state.store, &state.runtimes, &wav, config).await?;
    let import_ms = started.elapsed().as_millis();
    let playback = Instant::now();
    media::playback(&state.store, &state.runtimes, &meeting.id).await?;
    let playback_ms = playback.elapsed().as_millis();
    let mut inference_ms = None;
    let mut transcription_ms = None;
    let mut analysis_ms = None;
    if with_models {
        let start = Instant::now();
        eprintln!("Installed check: local transcription");
        jobs::refine_transcript(state, &meeting, || {}).await?;
        transcription_ms = Some(start.elapsed().as_millis());
        let current = state.store.load(&meeting.id)?;
        ensure!(
            current
                .transcript
                .iter()
                .any(|s| s.text.to_lowercase().contains("country")),
            "Offline ASR did not recognize the public fixture"
        );
        let analysis_start = Instant::now();
        eprintln!("Installed check: local summary");
        let analysis = state.ai.analyze(state, &current, false).await?;
        analysis_ms = Some(analysis_start.elapsed().as_millis());
        ensure!(
            !analysis.summary.is_empty(),
            "Offline model produced no summary"
        );
        analysis.validate_sources(&current.source_ids())?;
        state.store.update(&meeting.id, |m| {
            m.analysis = Some(analysis);
            Ok(())
        })?;
        inference_ms = Some(start.elapsed().as_millis());
    }
    let current = state.store.load(&meeting.id)?;
    let export = Instant::now();
    eprintln!("Installed check: exporting PDF");
    exports::export(&current, &state.store.root.join("test.pdf"), "pdf")?;
    ensure!(
        current.cloud_requests == 0 && current.config.offline && current.duration_ms >= 10_000,
        "Offline fixture failed"
    );
    Ok(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "durationMs": current.duration_ms,
        "startupMs": startup_ms,
        "importMs": import_ms,
        "playbackMs": playback_ms,
        "inferenceMs": inference_ms,
        "transcriptionMs": transcription_ms,
        "analysisMs": analysis_ms,
        "exportMs": export.elapsed().as_millis(),
        "cloudRequests": current.cloud_requests,
    }))
}
