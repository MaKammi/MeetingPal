use crate::{
    domain::*,
    providers::{AnalysisProvider, TranscriptionProvider, VisionProvider},
    AppState,
};
use anyhow::Result;
use std::{sync::Arc, time::Duration};
use tauri::Emitter;

pub fn enqueue_segment(state: &AppState, id: &str, segment: &MediaSegment) -> Result<()> {
    state.store.enqueue(&Job {
        id: format!("{id}:transcribe:{}", segment.id),
        meeting_id: id.into(),
        kind: "transcribe".into(),
        payload: segment.id.clone(),
        state: "pending".into(),
        attempts: 0,
        error: None,
    })
}
pub fn enqueue_final(state: &AppState, id: &str) -> Result<()> {
    let meeting = state.store.load(id)?;
    for segment in &meeting.media {
        enqueue_segment(state, id, segment)?;
    }
    state.store.enqueue(&Job {
        id: format!("{id}:finalize"),
        meeting_id: id.into(),
        kind: "finalize".into(),
        payload: String::new(),
        state: "pending".into(),
        attempts: 0,
        error: None,
    })?;
    state.store.update(id, |m| {
        m.status = "processing".into();
        Ok(())
    })?;
    Ok(())
}
pub fn start(state: Arc<AppState>, app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            match state.store.next_job() {
                Ok(Some(job)) => {
                    if job.kind == "finalize" {
                        if let Ok(jobs) = state.store.jobs(&job.meeting_id) {
                            if jobs.iter().any(|j| {
                                j.kind == "transcribe"
                                    && matches!(j.state.as_str(), "pending" | "running")
                            }) {
                                let _ = state.store.defer_job(&job.id);
                                tokio::time::sleep(Duration::from_millis(200)).await;
                                continue;
                            }
                        }
                    }
                    let result = process(&state, &job).await;
                    let retry = result
                        .as_ref()
                        .err()
                        .is_some_and(|e| e.to_string().starts_with("Retryable:"))
                        && job.attempts < 2;
                    let error = result.err().map(|e| e.to_string());
                    let _ = state.store.finish_job(&job.id, error.as_deref(), retry);
                    if let Some(error) = error {
                        if !retry {
                            let _ = state.store.update(&job.meeting_id, |m| {
                                if !matches!(m.status.as_str(), "recording" | "paused" | "starting")
                                {
                                    m.status = "needs_attention".into();
                                }
                                if !m.warnings.contains(&error) {
                                    m.warnings.push(error.clone());
                                }
                                Ok(())
                            });
                        }
                    }
                    let _ = app.emit("meeting-changed", &job.meeting_id);
                }
                _ => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
    });
}
async fn process(state: &AppState, job: &Job) -> Result<()> {
    let meeting = state.store.load(&job.meeting_id)?;
    match job.kind.as_str() {
        "transcribe" => {
            let segment = meeting
                .media
                .iter()
                .find(|m| m.id == job.payload)
                .ok_or_else(|| anyhow::anyhow!("Audio segment no longer exists"))?;
            let transcript = state.ai.transcribe(state, &meeting, segment).await?;
            state.store.update(&meeting.id, |m| {
                let prefix = format!("{}-", segment.id);
                let edited: Vec<_> = m
                    .transcript
                    .iter()
                    .filter(|t| t.id.starts_with(&prefix) && t.edited)
                    .cloned()
                    .collect();
                m.transcript.retain(|t| !t.id.starts_with(&prefix));
                for t in transcript {
                    m.transcript
                        .push(edited.iter().find(|e| e.id == t.id).cloned().unwrap_or(t));
                }
                m.transcript.sort_by_key(|t| t.start_ms);
                Ok(())
            })?;
        }
        "live_analysis" => {
            if matches!(meeting.status.as_str(), "recording" | "paused")
                && !meeting.transcript.is_empty()
            {
                let analysis = state.ai.analyze(state, &meeting, true).await?;
                state.store.archive_analysis(&meeting.id, &analysis)?;
                state.store.update(&meeting.id, |m| {
                    m.analysis = Some(analysis);
                    Ok(())
                })?;
            }
        }
        "finalize" => {
            anyhow::ensure!(
                !state
                    .store
                    .jobs(&meeting.id)?
                    .iter()
                    .any(|j| j.kind == "transcribe" && j.state == "failed"),
                "Some transcription jobs failed. Fix the reported issue and retry processing."
            );
            crate::media::playback(&state.store, &state.runtimes, &meeting.id).await?;
            if meeting.config.video_analysis != VideoAnalysis::None {
                let native = meeting.config.video_analysis == VideoAnalysis::Native;
                let frames = if native {
                    crate::media::native_clips(&state.store, &state.runtimes, &meeting).await?
                } else {
                    crate::media::extract_frames(&state.store, &state.runtimes, &meeting).await?
                };
                for (index, (file, at_ms)) in frames.iter().enumerate() {
                    let source_id = format!("visual-{index:06}");
                    if state
                        .store
                        .load(&meeting.id)?
                        .visuals
                        .iter()
                        .any(|v| v.id == source_id)
                    {
                        continue;
                    }
                    let text = state
                        .ai
                        .describe(state, &meeting, file, *at_ms, native)
                        .await?;
                    state.store.update(&meeting.id, |m| {
                        m.visuals.push(VisualEvidence {
                            id: source_id,
                            at_ms: *at_ms,
                            text,
                            file: file.clone(),
                        });
                        Ok(())
                    })?;
                }
            }
            let current = state.store.load(&meeting.id)?;
            let analysis = if current.transcript.is_empty() && current.visuals.is_empty() {
                AnalysisResult {
                    generated_at: chrono::Utc::now().to_rfc3339(),
                    ..Default::default()
                }
            } else {
                state.ai.analyze(state, &current, false).await?
            };
            state.store.archive_analysis(&meeting.id, &analysis)?;
            state.store.update(&meeting.id, |m| {
                m.analysis = Some(analysis);
                for t in &mut m.transcript {
                    t.provisional = false;
                }
                m.status = "complete".into();
                Ok(())
            })?;
        }
        _ => anyhow::bail!("Unknown processing job"),
    }
    Ok(())
}
