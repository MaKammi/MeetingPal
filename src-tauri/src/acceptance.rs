use crate::{domain::*, exports, jobs, media, providers::AnalysisProvider, AppState};
use anyhow::{ensure, Result};
use serde_json::{json, Value};
use std::time::Instant;

/// Exercise installed runtimes and persistence using the public fixture in a temporary library.
pub async fn offline(state: &AppState, with_models: bool) -> Result<Value> {
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
    let meeting = media::import(&state.store, &state.runtimes, &wav, config).await?;
    let import_ms = started.elapsed().as_millis();
    let playback = Instant::now();
    media::playback(&state.store, &state.runtimes, &meeting.id).await?;
    let playback_ms = playback.elapsed().as_millis();
    let mut inference_ms = None;
    if with_models {
        let start = Instant::now();
        jobs::refine_transcript(state, &meeting, || {}).await?;
        let current = state.store.load(&meeting.id)?;
        ensure!(
            current
                .transcript
                .iter()
                .any(|s| s.text.to_lowercase().contains("country")),
            "Offline ASR did not recognize the public fixture"
        );
        let analysis = state.ai.analyze(state, &current, false).await?;
        analysis.validate_sources(&current.source_ids())?;
        state.store.update(&meeting.id, |m| {
            m.analysis = Some(analysis);
            Ok(())
        })?;
        inference_ms = Some(start.elapsed().as_millis());
    }
    let current = state.store.load(&meeting.id)?;
    let export = Instant::now();
    exports::export(&current, &state.store.root.join("test.pdf"), "pdf")?;
    ensure!(
        current.cloud_requests == 0 && current.config.offline && current.duration_ms >= 10_000,
        "Offline fixture failed"
    );
    Ok(
        json!({"version":env!("CARGO_PKG_VERSION"),"os":std::env::consts::OS,"arch":std::env::consts::ARCH,"durationMs":current.duration_ms,"importMs":import_ms,"playbackMs":playback_ms,"inferenceMs":inference_ms,"exportMs":export.elapsed().as_millis(),"cloudRequests":current.cloud_requests}),
    )
}
