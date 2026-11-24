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
    if meeting.config.mode != Mode::Post {
        for segment in &meeting.media {
            enqueue_segment(state, id, segment)?;
        }
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
    let paused = state
        .store
        .jobs(id)?
        .iter()
        .any(|j| matches!(j.state.as_str(), "failed" | "paused"));
    state.store.update(id, |m| {
        m.status = if paused {
            "needs_attention"
        } else {
            "processing"
        }
        .into();
        Ok(())
    })?;
    Ok(())
}
pub fn start(state: Arc<AppState>, app: tauri::AppHandle) {
    for _ in 0..2 {
        let state = Arc::clone(&state);
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                match state.store.next_job() {
                    Ok(Some(job)) => {
                        let result = process(&state, &job, &app).await;
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
                                    if !matches!(
                                        m.status.as_str(),
                                        "recording" | "paused" | "starting"
                                    ) {
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
}
async fn process(state: &AppState, job: &Job, app: &tauri::AppHandle) -> Result<()> {
    let meeting = state.store.load(&job.meeting_id)?;
    match job.kind.as_str() {
        "transcribe" => {
            let segment = meeting
                .media
                .iter()
                .find(|m| m.id == job.payload)
                .ok_or_else(|| anyhow::anyhow!("Audio segment no longer exists"))?;
            let preview = |text: &str| {
                let _ = app.emit(
                    "transcript-preview",
                    serde_json::json!({"meetingId": meeting.id, "text": text}),
                );
            };
            let transcript = state
                .ai
                .transcribe_with_progress(state, &meeting, segment, preview)
                .await;
            if meeting.config.stream_transcription {
                preview("");
            }
            let transcript = transcript?;
            state.store.update(&meeting.id, |m| {
                let prefix = format!("{}-", segment.id);
                let edited: Vec<_> = m
                    .transcript
                    .iter()
                    .filter(|t| t.id.starts_with(&prefix) && t.edited)
                    .cloned()
                    .collect();
                m.transcript
                    .retain(|t| !t.id.starts_with(&prefix) || t.edited);
                m.transcript.extend(transcript.into_iter().filter(|t| {
                    !edited
                        .iter()
                        .any(|e| e.id == t.id || (e.start_ms < t.end_ms && t.start_ms < e.end_ms))
                }));
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
            crate::media::playback(&state.store, &state.runtimes, &meeting.id).await?;
            refine_transcript(state, &meeting, || {
                let _ = app.emit("meeting-changed", &meeting.id);
            })
            .await?;
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

/// Larger context in the final pass; cache each successful section before moving on.
/// Edited source sections keep their original IDs and wording.
pub async fn refine_transcript(
    state: &AppState,
    meeting: &Meeting,
    progress: impl Fn(),
) -> Result<()> {
    use crate::storage::{atomic_json, read_json};
    let folder = state.store.folder(&meeting.id)?;
    let mut final_config = meeting.clone();
    final_config.status = "processing".into();
    if let Some(selection) = &meeting.config.final_transcription {
        final_config.config.transcription = selection.clone();
    }
    // Speaker IDs are request-scoped. Use up to ten minutes of 16 kHz mono PCM
    // (19.2 MB) per diarization request while keeping existing ASR cache boundaries.
    let diarized = crate::transcription::model_supports(
        final_config.config.transcription.provider.key(),
        &final_config.config.transcription.model,
        "speakerLabels",
    );
    let mut pending = meeting.media.as_slice();
    let groups = std::iter::from_fn(move || {
        let first = pending.first()?;
        let count = if diarized {
            pending
                .partition_point(|s| s.end_ms.saturating_sub(first.start_ms) <= 600_000)
                .max(1)
        } else {
            pending.len().min(12)
        };
        let (group, rest) = pending.split_at(count);
        pending = rest;
        Some(group)
    });
    for (index, group) in groups.enumerate() {
        let start_ms = group.first().unwrap().start_ms;
        let end_ms = group.last().unwrap().end_ms;
        let current = state.store.load(&meeting.id)?;
        if current
            .transcript
            .iter()
            .any(|s| s.edited && s.start_ms < end_ms && s.end_ms > start_ms)
        {
            continue;
        }
        let identity = serde_json::json!({"selection":final_config.config.transcription,"language":meeting.config.language,"segments":group});
        let cached_path = folder.join(format!("refinement-{index:06}.json"));
        let cached: Option<serde_json::Value> = read_json(&cached_path).ok();
        let transcript: Vec<TranscriptSegment> =
            if let Some(cache) = cached.filter(|c| c["identity"] == identity) {
                serde_json::from_value(cache["transcript"].clone())?
            } else {
                let file = format!("final-audio-{index:06}.wav");
                crate::media::concat_wav(&folder, group, &folder.join(&file))?;
                let section = MediaSegment {
                    id: format!("final-{index:06}"),
                    start_ms,
                    end_ms,
                    microphone: None,
                    desktop: None,
                    mixed: file,
                };
                final_config.transcript = current.transcript.clone();
                let transcript = state.ai.transcribe(state, &final_config, &section).await?;
                atomic_json(
                    &cached_path,
                    &serde_json::json!({"identity":identity,"transcript":transcript}),
                )?;
                transcript
            };
        state.store.update(&meeting.id, |m| {
            // A user may have edited this region while inference was running.
            if !m
                .transcript
                .iter()
                .any(|s| s.edited && s.start_ms < end_ms && s.end_ms > start_ms)
            {
                m.transcript
                    .retain(|s| s.start_ms >= end_ms || s.end_ms <= start_ms);
                m.transcript.extend(transcript);
                m.transcript.sort_by_key(|s| s.start_ms);
            }
            if let Some(analysis) = &mut m.analysis {
                analysis.provisional = true;
            }
            Ok(())
        })?;
        progress();
    }
    Ok(())
}
