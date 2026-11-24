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

#[cfg(test)]
#[test]
fn a_manual_edit_protects_its_audio_and_keeps_the_other_sections_available() {
    let media: Vec<_> = (0..5)
        .map(|i| MediaSegment {
            id: i.to_string(),
            start_ms: i * 5000,
            end_ms: (i + 1) * 5000,
            mixed: String::new(),
            microphone: None,
            desktop: None,
        })
        .collect();
    let mut edit = TranscriptSegment {
        id: "live-2".into(),
        start_ms: 11_000,
        end_ms: 13_000,
        text: "Corrected".into(),
        speaker: None,
        provisional: false,
        edited: true,
    };
    let sections = unedited_sections("000000", &media, std::slice::from_ref(&edit)).unwrap();
    assert_eq!(
        sections
            .iter()
            .map(|(id, part)| (id.as_str(), part.len()))
            .collect::<Vec<_>>(),
        [("000000_0_10000", 2), ("000000_15000_25000", 2)]
    );
    edit.id = "final-000000-2".into();
    assert!(
        unedited_sections("000000", &media, std::slice::from_ref(&edit))
            .unwrap()
            .is_empty()
    );
    let separated = [media[0].clone(), media[4].clone()];
    assert!(unedited_sections("000000", &separated, &[edit]).is_none());
}

fn overlaps_media(row: &TranscriptSegment, group: &[MediaSegment]) -> bool {
    let index = group.partition_point(|media| media.end_ms <= row.start_ms);
    group
        .get(index)
        .is_some_and(|media| media.start_ms < row.end_ms)
}

/// Return the untouched runs of source audio when a section contains a manual correction.
fn unedited_sections<'a>(
    section: &str,
    group: &'a [MediaSegment],
    transcript: &[TranscriptSegment],
) -> Option<Vec<(String, &'a [MediaSegment])>> {
    let edits: Vec<_> = transcript
        .iter()
        .filter(|s| s.edited && overlaps_media(s, group))
        .collect();
    if edits.is_empty() {
        return None;
    }
    // The whole section was already successfully applied before the user edited its result.
    let final_prefix = format!("final-{section}-");
    if edits.iter().any(|s| s.id.starts_with(&final_prefix)) {
        return Some(Vec::new());
    }
    let base = section.split('_').next().unwrap_or(section);
    let parts = group
        .split(|media| {
            edits
                .iter()
                .any(|s| s.start_ms < media.end_ms && media.start_ms < s.end_ms)
        })
        .filter(|part| !part.is_empty())
        .map(|part| {
            let start = part[0].start_ms;
            let end = part[part.len() - 1].end_ms;
            (format!("{base}_{start}_{end}"), part)
        })
        .collect();
    Some(parts)
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
    // Leave room for base64 encoding Gemini WAVs inside an inline JSON request.
    // OpenAI accepts the same 16 kHz mono PCM as a binary multipart attachment.
    let max_section_ms = if final_config.config.transcription.provider == Provider::Gemini {
        300_000
    } else {
        600_000
    };
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
                .partition_point(|s| s.end_ms.saturating_sub(first.start_ms) <= max_section_ms)
                .max(1)
        } else {
            pending.len().min(12)
        };
        let (group, rest) = pending.split_at(count);
        pending = rest;
        Some(group)
    });
    let mut sections: std::collections::VecDeque<_> = groups
        .enumerate()
        .map(|(index, group)| (format!("{index:06}"), group))
        .collect();
    while let Some((section_id, group)) = sections.pop_front() {
        let start_ms = group.first().unwrap().start_ms;
        let end_ms = group.last().unwrap().end_ms;
        let current = state.store.load(&meeting.id)?;
        if let Some(parts) = unedited_sections(&section_id, group, &current.transcript) {
            for part in parts.into_iter().rev() {
                sections.push_front(part);
            }
            continue;
        }
        let identity = serde_json::json!({"selection":final_config.config.transcription,"language":meeting.config.language,"segments":group});
        let cached_path = folder.join(format!("refinement-{section_id}.json"));
        let cached: Option<serde_json::Value> = read_json(&cached_path).ok();
        let transcript: Vec<TranscriptSegment> =
            if let Some(cache) = cached.filter(|c| c["identity"] == identity) {
                serde_json::from_value(cache["transcript"].clone())?
            } else {
                let file = format!("final-audio-{section_id}.wav");
                crate::media::concat_wav(&folder, group, &folder.join(&file))?;
                let section = MediaSegment {
                    id: format!("final-{section_id}"),
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
        let mut applied = false;
        state.store.update(&meeting.id, |m| {
            // A correction made during inference also protects its underlying source audio.
            if !m
                .transcript
                .iter()
                .any(|s| s.edited && overlaps_media(s, group))
            {
                m.transcript.retain(|s| !overlaps_media(s, group));
                m.transcript.extend(transcript);
                m.transcript.sort_by_key(|s| s.start_ms);
                if let Some(analysis) = &mut m.analysis {
                    analysis.provisional = true;
                }
                applied = true;
            }
            Ok(())
        })?;
        if applied {
            progress();
        } else {
            sections.push_front((section_id, group));
        }
    }
    Ok(())
}
