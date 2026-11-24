use crate::{
    capture::{CaptureBackend, CaptureEvent, NativeCapture, WorkerSpec},
    domain::*,
    jobs, providers,
    storage::{atomic_json, safe_child},
    ActiveRecording, AppState,
};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tauri::{Emitter, State};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

type Response<T> = Result<T, String>;
fn message(error: impl std::fmt::Display) -> String {
    error.to_string()
}
fn changed(app: &tauri::AppHandle, id: &str) {
    let _ = app.emit("meeting-changed", id);
}
#[tauri::command]
pub async fn bootstrap(state: State<'_, Arc<AppState>>) -> Response<Value> {
    Ok(
        json!({"settings":state.store.settings(),"meetings":state.store.list("").map_err(message)?,"models":state.models.catalog(),"runtimes":state.runtimes.status(),"keys":state.secrets.status(),"capabilities":providers::capabilities(),"dataDirectory":state.store.root,"activeMeeting":state.recording.lock().await.as_ref().map(|r|r.meeting_id.clone())}),
    )
}
#[tauri::command]
pub fn list_meetings(
    state: State<'_, Arc<AppState>>,
    query: String,
) -> Response<Vec<MeetingSummary>> {
    state.store.list(&query).map_err(message)
}
#[tauri::command]
pub fn get_meeting(state: State<'_, Arc<AppState>>, id: String) -> Response<Meeting> {
    state.store.load(&id).map_err(message)
}
#[tauri::command]
pub fn get_jobs(state: State<'_, Arc<AppState>>, id: String) -> Response<Vec<Job>> {
    state.store.jobs(&id).map_err(message)
}
#[tauri::command]
pub async fn capture_sources() -> Response<Value> {
    tokio::task::spawn_blocking(|| {
        let devices=NativeCapture.devices();let sources=NativeCapture.sources();
        json!({"devices":devices.as_ref().ok(),"sources":sources.as_ref().ok(),"deviceError":devices.err().map(message),"sourceError":sources.err().map(message)})
    }).await.map_err(message)
}
#[tauri::command]
pub async fn capture_source_preview(
    _window: tauri::Window,
    kind: VideoKind,
    id: String,
    detail: bool,
) -> Response<crate::capture::SourcePreview> {
    static SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
    #[cfg(target_os = "windows")]
    static PREVIEW_EXCLUSION: std::sync::RwLock<()> = std::sync::RwLock::new(());
    let slot = SLOTS.acquire().await.map_err(message)?;
    tokio::task::spawn_blocking(move || {
        let _slot = slot;
        #[cfg(target_os = "windows")]
        {
            // Screen thumbnails should show what is behind the picker. Keep window
            // thumbnails (including MeetingPal itself) outside this brief exclusion.
            if kind == VideoKind::Screen {
                let _exclusive = PREVIEW_EXCLUSION.write().unwrap_or_else(|e| e.into_inner());
                struct Restore(tauri::Window);
                impl Drop for Restore {
                    fn drop(&mut self) {
                        let _ = self.0.set_content_protected(false);
                    }
                }
                _window.set_content_protected(true)?;
                let _restore = Restore(_window);
                return crate::capture::source_preview(kind, id, detail);
            }
        }
        #[cfg(target_os = "windows")]
        let _window_read = PREVIEW_EXCLUSION.read().unwrap_or_else(|e| e.into_inner());
        crate::capture::source_preview(kind, id, detail)
    })
    .await
    .map_err(message)?
    .map_err(message)
}
#[tauri::command]
pub async fn list_video_sources() -> Response<Vec<crate::capture::Source>> {
    tokio::task::spawn_blocking(|| {
        #[cfg(target_os = "macos")]
        if !scap::has_permission() {
            scap::request_permission();
        }
        NativeCapture.sources()
    })
    .await
    .map_err(message)?
    .map_err(message)
}
#[tauri::command]
pub fn save_settings(state: State<'_, Arc<AppState>>, mut settings: Settings) -> Response<()> {
    if !["de", "en"].contains(&settings.locale.as_str()) {
        return Err("Unsupported interface language".into());
    }
    if settings.retention_days.is_some_and(|d| d == 0 || d > 3650) {
        return Err("Retention must be between 1 and 3650 days".into());
    }
    settings.default_config.consent = false;
    state.store.save_settings(&settings).map_err(message)
}
#[tauri::command]
pub fn save_key(
    state: State<'_, Arc<AppState>>,
    provider: Provider,
    key: String,
) -> Response<bool> {
    state.secrets.set(&provider, key).map_err(message)
}
#[tauri::command]
pub fn delete_key(state: State<'_, Arc<AppState>>, provider: Provider) -> Response<()> {
    state.secrets.delete(&provider).map_err(message)
}
#[tauri::command]
pub async fn provider_models(
    state: State<'_, Arc<AppState>>,
    provider: Provider,
) -> Response<Vec<String>> {
    state
        .ai
        .list_models(&state, &provider)
        .await
        .map_err(message)
}
#[tauri::command]
pub async fn probe_model(
    state: State<'_, Arc<AppState>>,
    selection: ModelSelection,
    task: crate::model_probe::Task,
) -> Response<crate::model_probe::Report> {
    let _work = state.updates.working()?;
    crate::model_probe::run(
        state.secrets.clone(),
        state.runtimes.clone(),
        selection,
        task,
    )
    .await
    .map_err(message)
}
#[tauri::command]
pub async fn download_model(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
) -> Response<()> {
    let _work = state.updates.working()?;
    state.models.download(id, &app).await.map_err(message)
}
#[tauri::command]
pub fn cancel_download(state: State<'_, Arc<AppState>>, id: String) {
    state.models.cancel(&id);
}
#[tauri::command]
pub async fn delete_model(state: State<'_, Arc<AppState>>, id: String) -> Response<()> {
    let _work = state.updates.working()?;
    if state.recording.lock().await.is_some() {
        return Err("Stop recording before removing models".into());
    }
    if state.store.has_active_jobs().map_err(message)? {
        return Err("Wait for queued AI processing to finish before removing models.".into());
    }
    state.ai.shutdown().await;
    state.models.remove(&id).map_err(message)
}
#[tauri::command]
pub async fn benchmark_model(state: State<'_, Arc<AppState>>, id: String) -> Response<Value> {
    let _work = state.updates.working()?;
    let spec = state.models.spec(&id).map_err(message)?;
    if spec.task != "transcription" {
        return Err("The benchmark currently measures speech recognition.".into());
    }
    let model = state.models.path(&id).map_err(message)?;
    let sample = state.store.root.join("benchmark.wav");
    std::fs::write(&sample, include_bytes!("../../resources/benchmark.wav")).map_err(message)?;
    let reader = hound::WavReader::open(&sample).map_err(message)?;
    let duration = reader.duration() as f64 / reader.spec().sample_rate as f64;
    let start = std::time::Instant::now();
    crate::media::run(
        &state.runtimes.get("whisper-cli").map_err(message)?,
        &[
            "-m".into(),
            model.to_string_lossy().into(),
            "-f".into(),
            sample.to_string_lossy().into(),
            "-l".into(),
            "en".into(),
            "-np".into(),
        ],
        Duration::from_secs(300),
    )
    .await
    .map_err(message)?;
    Ok(
        json!({"audioSeconds":duration,"processingSeconds":start.elapsed().as_secs_f64(),"realtimeFactor":start.elapsed().as_secs_f64()/duration,"includesModelLoading":true}),
    )
}
#[tauri::command]
pub async fn start_recording(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    config: MeetingConfig,
    test: bool,
) -> Response<Meeting> {
    let _work = state.updates.working()?;
    start_recording_inner(state.inner().clone(), app, config, test)
        .await
        .map_err(message)
}
async fn start_recording_inner(
    state: Arc<AppState>,
    app: tauri::AppHandle,
    config: MeetingConfig,
    test: bool,
) -> anyhow::Result<Meeting> {
    config.validate()?;
    let mut guard = state.recording.lock().await;
    anyhow::ensure!(guard.is_none(), "A recording is already running.");
    if config.mode != Mode::Post && !test {
        if config.transcription.provider == Provider::Local {
            state.models.path(&config.transcription.model)?;
            state.runtimes.get("whisper-cli")?;
        } else {
            state.secrets.get(&config.transcription.provider)?;
        }
        if config.mode == Mode::Assistant {
            if config.analysis.provider == Provider::Local {
                state.models.path(&config.analysis.model)?;
                state.runtimes.get("llama-server")?;
            } else {
                state.secrets.get(&config.analysis.provider)?;
            }
        }
    }
    let ffmpeg = if config.video.kind == VideoKind::None {
        None
    } else {
        Some(state.runtimes.get("ffmpeg")?)
    };
    let mut command: tokio::process::Command =
        crate::capture::hidden_command(std::env::current_exe()?).into();
    #[cfg(target_os = "linux")]
    let portal_grant = {
        use tauri::Manager;
        if let Some(id) = config
            .video
            .id
            .as_deref()
            .filter(|id| config.video.kind == VideoKind::Portal && id.starts_with("portal:"))
        {
            let grant = app
                .state::<crate::capture_linux::PortalSources>()
                .lock()
                .unwrap()
                .get(id)
                .cloned()
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "The system capture session has ended. Select the source again."
                    )
                })?;
            grant.attach(&mut command).await?;
            Some(grant)
        } else {
            None
        }
    };
    let mut meeting = Meeting::new(config.clone());
    meeting.status = "starting".into();
    if test {
        meeting.tags.push("audio-test".into());
    }
    state.store.save(&meeting)?;
    let folder = state.store.folder(&meeting.id)?;
    let spec = WorkerSpec {
        folder: folder.clone(),
        config,
        ffmpeg,
        test_seconds: if test { Some(5) } else { None },
    };
    let spec_path = folder.join("worker.json");
    atomic_json(&spec_path, &spec)?;
    let mut child = command
        .arg("--capture-worker")
        .arg(spec_path)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::fs::File::create(folder.join("capture-worker.log"))?)
        .kill_on_drop(true)
        .spawn()?;
    let stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    *guard = Some(ActiveRecording {
        meeting_id: meeting.id.clone(),
        stdin,
        paused: false,
    });
    drop(guard);
    let id = meeting.id.clone();
    tauri::async_runtime::spawn(async move {
        // The setup dialog can release its copy once the recorder is running.
        #[cfg(target_os = "linux")]
        let _grant = portal_grant;
        let mut lines = tokio::io::BufReader::new(stdout).lines();
        let mut stopped = false;
        while let Ok(Some(line)) = lines.next_line().await {
            let Ok(event) = serde_json::from_str::<CaptureEvent>(&line) else {
                continue;
            };
            let _ = app.emit("capture-event", json!({"meetingId":id,"event":event}));
            match event {
                CaptureEvent::Ready => {
                    let _ = state.store.update(&id, |m| {
                        m.status = "recording".into();
                        Ok(())
                    });
                }
                CaptureEvent::Level { .. } => {}
                CaptureEvent::State { paused } => {
                    if let Some(active) = state.recording.lock().await.as_mut() {
                        active.paused = paused;
                    }
                    let _ = state.store.update(&id, |m| {
                        m.status = if paused { "paused" } else { "recording" }.into();
                        Ok(())
                    });
                }
                CaptureEvent::Warning { message } => {
                    let _ = state.store.update(&id, |m| {
                        if !m.warnings.contains(&message) {
                            m.warnings.push(message);
                        }
                        Ok(())
                    });
                }
                CaptureEvent::Segment { segment } => {
                    if let Ok(m) = state.store.update(&id, |m| {
                        if !m.media.iter().any(|s| s.id == segment.id) {
                            m.media.push(segment.clone());
                        }
                        m.duration_ms = segment.end_ms;
                        Ok(())
                    }) {
                        if !test && m.config.mode != Mode::Post {
                            let _ = jobs::enqueue_segment(&state, &id, &segment);
                        }
                        if !test && m.config.mode == Mode::Assistant && m.media.len() % 12 == 0 {
                            let job = Job {
                                id: format!("{id}:live:{}", m.media.len()),
                                meeting_id: id.clone(),
                                kind: "live_analysis".into(),
                                payload: String::new(),
                                state: "pending".into(),
                                attempts: 0,
                                error: None,
                            };
                            let _ = state.store.enqueue(&job);
                        }
                    }
                }
                CaptureEvent::Stopped { manifest } => {
                    stopped = true;
                    let _ = state.store.update(&id, |m| {
                        m.media = manifest.segments;
                        m.video_files = manifest.video_files;
                        m.duration_ms = manifest.duration_ms;
                        m.status = "recorded".into();
                        Ok(())
                    });
                    if test {
                        if let Err(e) =
                            crate::media::playback(&state.store, &state.runtimes, &id).await
                        {
                            let _ = state.store.update(&id, |m| {
                                m.warnings.push(e.to_string());
                                Ok(())
                            });
                        }
                    } else {
                        let _ = jobs::enqueue_final(&state, &id);
                    }
                }
            }
            changed(&app, &id);
        }
        let status = child.wait().await;
        if !stopped {
            let _=state.store.update(&id,|m|{
                if let Ok(manifest)=crate::storage::read_json::<crate::capture::CaptureManifest>(&folder.join("capture.json")){m.media=manifest.segments;m.duration_ms=manifest.duration_ms;m.video_files=manifest.video_files;}
                m.status="needs_attention".into();m.warnings.push(format!("Capture process stopped unexpectedly ({status:?}). Completed segments are preserved. Check capture-worker.log."));Ok(())
            });
        }
        let mut active = state.recording.lock().await;
        if active.as_ref().is_some_and(|r| r.meeting_id == id) {
            *active = None;
        }
        drop(active);
        changed(&app, &id);
        let _ = app.emit("recording-ended", &id);
    });
    Ok(meeting)
}
pub async fn control(state: &AppState, action: &str) -> anyhow::Result<()> {
    let mut guard = state.recording.lock().await;
    let active = guard
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("No active recording"))?;
    let action = if action == "toggle" {
        if active.paused {
            "resume"
        } else {
            "pause"
        }
    } else {
        action
    };
    anyhow::ensure!(
        ["stop", "pause", "resume"].contains(&action),
        "Unknown recording action"
    );
    active
        .stdin
        .write_all(format!("{action}\n").as_bytes())
        .await?;
    active.stdin.flush().await?;
    if action != "stop" {
        active.paused = action == "pause";
    }
    Ok(())
}
#[tauri::command]
pub async fn control_recording(state: State<'_, Arc<AppState>>, action: String) -> Response<()> {
    control(&state, &action).await.map_err(message)
}
#[tauri::command]
pub fn add_marker(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    at_ms: u64,
    text: String,
) -> Response<Meeting> {
    let m = state
        .store
        .update(&id, |m| {
            anyhow::ensure!(text.len() <= 2000, "Marker is too long");
            m.markers.push(Marker { at_ms, text });
            Ok(())
        })
        .map_err(message)?;
    changed(&app, &id);
    Ok(m)
}
#[tauri::command]
pub fn update_notes(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    title: String,
    tags: Vec<String>,
    notes: String,
) -> Response<Meeting> {
    let m = state
        .store
        .update(&id, |m| {
            anyhow::ensure!(
                !title.trim().is_empty()
                    && title.len() <= 250
                    && notes.len() <= 1_000_000
                    && tags.len() <= 50,
                "Invalid note metadata"
            );
            m.config.title = title;
            m.tags = tags;
            m.notes = notes;
            Ok(())
        })
        .map_err(message)?;
    changed(&app, &id);
    Ok(m)
}
#[tauri::command]
pub fn edit_transcript(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    segment_id: String,
    text: String,
    speaker: Option<String>,
) -> Response<Meeting> {
    let m = state
        .store
        .update(&id, |m| {
            anyhow::ensure!(text.len() <= 10000, "Transcript edit is too long");
            let segment = m
                .transcript
                .iter_mut()
                .find(|s| s.id == segment_id)
                .ok_or_else(|| anyhow::anyhow!("Transcript segment no longer exists"))?;
            segment.text = text;
            segment.speaker = speaker;
            segment.edited = true;
            if let Some(a) = &mut m.analysis {
                a.provisional = true;
            }
            Ok(())
        })
        .map_err(message)?;
    changed(&app, &id);
    Ok(m)
}
#[tauri::command]
pub async fn analyze_again(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
) -> Response<Meeting> {
    let _work = state.updates.working()?;
    let current = state.store.load(&id).map_err(message)?;
    let result = crate::providers::AnalysisProvider::analyze(&state.ai, &state, &current, false)
        .await
        .map_err(message)?;
    state
        .store
        .archive_analysis(&id, &result)
        .map_err(message)?;
    let m = state
        .store
        .update(&id, |m| {
            m.analysis = Some(result);
            Ok(())
        })
        .map_err(message)?;
    changed(&app, &id);
    Ok(m)
}
#[tauri::command]
pub async fn ask_meeting(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    question: String,
) -> Response<Meeting> {
    let _work = state.updates.working()?;
    let current = state.store.load(&id).map_err(message)?;
    let answer = state
        .ai
        .answer(&state, &current, &question)
        .await
        .map_err(message)?;
    let m = state
        .store
        .update(&id, |m| {
            m.answers.push(Answer {
                question,
                answer,
                created_at: chrono::Utc::now().to_rfc3339(),
            });
            Ok(())
        })
        .map_err(message)?;
    changed(&app, &id);
    Ok(m)
}
#[tauri::command]
pub fn retry_processing(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    max_requests: u32,
) -> Response<()> {
    let _work = state.updates.working()?;
    if max_requests == 0 || max_requests > 100000 {
        return Err("Invalid request budget".into());
    }
    state
        .store
        .update(&id, |m| {
            m.config.max_requests = max_requests;
            Ok(())
        })
        .map_err(message)?;
    state.store.retry_jobs(&id).map_err(message)?;
    if !matches!(
        state.store.load(&id).map_err(message)?.status.as_str(),
        "recording" | "paused" | "starting"
    ) {
        jobs::enqueue_final(&state, &id).map_err(message)?;
    }
    changed(&app, &id);
    Ok(())
}
#[tauri::command]
pub async fn import_media(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    path: String,
    config: MeetingConfig,
) -> Response<Meeting> {
    let _work = state.updates.working()?;
    let meeting = crate::media::import(&state.store, &state.runtimes, &PathBuf::from(path), config)
        .await
        .map_err(message)?;
    jobs::enqueue_final(&state, &meeting.id).map_err(message)?;
    changed(&app, &meeting.id);
    Ok(meeting)
}
#[tauri::command]
pub async fn make_playback(state: State<'_, Arc<AppState>>, id: String) -> Response<String> {
    let _work = state.updates.working()?;
    crate::media::playback(&state.store, &state.runtimes, &id)
        .await
        .map_err(message)
}
#[tauri::command]
pub fn media_path(state: State<'_, Arc<AppState>>, id: String, file: String) -> Response<String> {
    if !["wav", "webm", "jpg", "png", "mp4"].contains(
        &PathBuf::from(&file)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default(),
    ) {
        return Err("Not a playable media file".into());
    }
    let path = safe_child(&state.store.folder(&id).map_err(message)?, &file).map_err(message)?;
    if !path.is_file() {
        return Err("Media file not found".into());
    }
    Ok(path.to_string_lossy().into())
}
#[tauri::command]
pub async fn delete_meeting(
    state: State<'_, Arc<AppState>>,
    app: tauri::AppHandle,
    id: String,
    media_only: bool,
) -> Response<()> {
    let _work = state.updates.working()?;
    if state
        .recording
        .lock()
        .await
        .as_ref()
        .is_some_and(|r| r.meeting_id == id)
    {
        return Err("Stop the recording before deleting it.".into());
    }
    if state
        .store
        .jobs(&id)
        .map_err(message)?
        .iter()
        .any(|j| matches!(j.state.as_str(), "pending" | "running"))
    {
        return Err(
            "Processing is still running. Wait for it to finish before deleting this meeting."
                .into(),
        );
    }
    state.store.delete(&id, media_only).map_err(message)?;
    changed(&app, &id);
    Ok(())
}
#[tauri::command]
pub fn export_meeting(
    state: State<'_, Arc<AppState>>,
    id: String,
    path: String,
    format: String,
) -> Response<()> {
    let _work = state.updates.working()?;
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("Choose an absolute export path".into());
    }
    let meeting = state.store.load(&id).map_err(message)?;
    crate::exports::export(&meeting, &path, &format).map_err(message)
}
