mod acceptance;
pub mod capture;
#[cfg(target_os = "linux")]
mod capture_linux;
#[cfg(target_os = "macos")]
mod capture_mac_audio;
pub mod commands;
pub mod domain;
pub mod exports;
mod gemini_files;
pub mod jobs;
pub mod media;
pub mod model_probe;
pub mod models;
pub mod providers;
pub mod secrets;
pub mod storage;
mod transcription;
pub mod updates;
use std::sync::Arc;
use tauri::{Emitter, Manager};
pub struct ActiveRecording {
    pub meeting_id: String,
    pub stdin: tokio::process::ChildStdin,
    pub paused: bool,
}
pub struct AppState {
    pub store: storage::Store,
    pub models: models::Models,
    pub secrets: secrets::Secrets,
    pub runtimes: media::Runtimes,
    pub ai: providers::Ai,
    pub recording: tokio::sync::Mutex<Option<ActiveRecording>>,
    pub updates: updates::Updates,
}
pub fn entry() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--capture-worker") {
        let result = capture::run_worker(std::path::Path::new(
            args.get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing worker specification"))?,
        ));
        if let Err(error) = &result {
            capture::emit(capture::CaptureEvent::Warning {
                message: format!("Recording failed: {error:#}"),
            });
        }
        return result;
    }
    // The supervising process owns this temporary directory. All WebView and SQLite
    // handles in the installed product have closed before Windows removes the files.
    if args.get(1).is_some_and(|arg| arg == "--check-offline") {
        anyhow::ensure!(
            (3..=4).contains(&args.len()),
            "Usage: --check-offline NEW_REPORT_PATH [INSTALLED_MODELS_DIRECTORY]"
        );
        let directory = tempfile::tempdir()?;
        let status = std::process::Command::new(tauri::utils::platform::current_exe()?)
            .arg("--check-offline-worker")
            .arg(directory.path().join("library"))
            .args(&args[2..])
            .status()?;
        directory.close()?;
        anyhow::ensure!(
            status.success(),
            "Installed offline check failed ({status})"
        );
        return Ok(());
    }
    let check = if args
        .get(1)
        .is_some_and(|arg| arg == "--check-offline-worker")
    {
        anyhow::ensure!(
            (4..=5).contains(&args.len()),
            "Missing installed-check arguments"
        );
        let root = std::path::PathBuf::from(&args[2]);
        std::fs::create_dir(&root)?;
        let report = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args[3])?;
        Some((root, report, args.get(4).map(std::path::PathBuf::from)))
    } else {
        None
    };
    let checking = check.is_some();
    let launch = std::time::Instant::now();
    let ready = check.as_ref().map(|_| Arc::new(tokio::sync::Notify::new()));
    let app = tauri::Builder::default()
        .manage(acceptance::Startup(ready.clone()))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        let state = app.state::<Arc<AppState>>().inner().clone();
                        tauri::async_runtime::spawn(async move {
                            let _ = commands::control(&state, "toggle").await;
                        });
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let root = match &check {
                Some((dir, _, _)) => dir.clone(),
                None => app.path().app_data_dir()?,
            };
            let model_root = check
                .as_ref()
                .and_then(|(_, _, models)| models.clone())
                .unwrap_or_else(|| root.join("models"));
            let directory = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
            } else {
                app.path().resource_dir()?.join("binaries")
            };
            let state = Arc::new(AppState {
                models: models::Models::new(model_root),
                store: storage::Store::open(root)?,
                secrets: secrets::Secrets::default(),
                runtimes: media::Runtimes { directory },
                ai: providers::Ai::new()?,
                recording: tokio::sync::Mutex::new(None),
                updates: updates::Updates::default(),
            });
            if let Some(days) = state.store.settings().retention_days {
                let threshold = chrono::Utc::now() - chrono::Duration::days(days as i64);
                for meeting in state.store.list("")? {
                    if meeting.status == "complete"
                        && chrono::DateTime::parse_from_rfc3339(&meeting.created_at)
                            .is_ok_and(|d| d < threshold)
                    {
                        let _ = state.store.delete(&meeting.id, false);
                    }
                }
            }
            app.manage(state.clone());
            #[cfg(target_os = "linux")]
            app.manage(capture_linux::PortalSources::default());
            if let Some((_, mut report, models)) = check {
                let ready = ready.expect("installed check startup notification");
                let handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    // Startup is part of the measured product path. Fail if the packaged WebView never bootstraps.
                    let result = async {
                        tokio::time::timeout(std::time::Duration::from_secs(60), ready.notified())
                            .await
                            .map_err(|_| {
                                anyhow::anyhow!(
                                    "Packaged WebView did not bootstrap within 60 seconds"
                                )
                            })?;
                        let startup_ms = launch.elapsed().as_millis();
                        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                        acceptance::offline(&state, models.is_some(), startup_ms).await
                    }
                    .await;
                    state.ai.shutdown().await;
                    let code = if result.is_ok() { 0 } else { 1 };
                    let value =
                        result.unwrap_or_else(|e| serde_json::json!({"error":format!("{e:#}")}));
                    let written = serde_json::to_writer_pretty(&mut report, &value).is_ok()
                        && report.sync_all().is_ok();
                    drop(state);
                    handle.exit(if written { code } else { 1 });
                });
                return Ok(());
            }
            jobs::start(state, app.handle().clone());
            use tauri_plugin_global_shortcut::GlobalShortcutExt;
            if let Err(e) = app
                .global_shortcut()
                .register("CommandOrControl+Shift+Space")
            {
                eprintln!("Global pause shortcut unavailable: {e}");
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<Arc<AppState>>();
                if state
                    .recording
                    .try_lock()
                    .map(|r| r.is_some())
                    .unwrap_or(true)
                {
                    api.prevent_close();
                    let _ = window.emit(
                        "app-notice",
                        "Please stop the recording before closing MeetingPal.",
                    );
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            updates::check_for_update,
            updates::install_update,
            commands::list_meetings,
            commands::get_meeting,
            commands::get_jobs,
            commands::capture_sources,
            commands::capture_source_preview,
            commands::list_video_sources,
            #[cfg(target_os = "linux")]
            capture_linux::select_portal_source,
            #[cfg(target_os = "linux")]
            capture_linux::release_portal_source,
            commands::save_settings,
            commands::save_key,
            commands::delete_key,
            commands::provider_models,
            commands::probe_model,
            commands::download_model,
            commands::cancel_download,
            commands::delete_model,
            commands::benchmark_model,
            commands::start_recording,
            commands::control_recording,
            commands::add_marker,
            commands::update_notes,
            commands::edit_transcript,
            commands::analyze_again,
            commands::ask_meeting,
            commands::retry_processing,
            commands::import_media,
            commands::make_playback,
            commands::media_path,
            commands::delete_meeting,
            commands::export_meeting
        ])
        .build(tauri::generate_context!())?;
    if checking {
        let code = app.run_return(|_, _| {});
        anyhow::ensure!(code == 0, "Installed offline check failed");
    } else {
        app.run(|_, _| {});
    }
    Ok(())
}
