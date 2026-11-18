pub mod capture;
#[cfg(target_os = "macos")]
mod capture_mac_audio;
pub mod commands;
pub mod domain;
pub mod exports;
pub mod jobs;
pub mod media;
pub mod models;
pub mod providers;
pub mod secrets;
pub mod storage;
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
}
pub fn entry() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--capture-worker") {
        return capture::run_worker(std::path::Path::new(
            args.get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing worker specification"))?,
        ));
    }
    tauri::Builder::default()
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
        .setup(|app| {
            let root = app.path().app_data_dir()?;
            let directory = if cfg!(debug_assertions) {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries")
            } else {
                app.path().resource_dir()?.join("binaries")
            };
            let state = Arc::new(AppState {
                models: models::Models::new(root.join("models")),
                store: storage::Store::open(root)?,
                secrets: secrets::Secrets::default(),
                runtimes: media::Runtimes { directory },
                ai: providers::Ai::new()?,
                recording: tokio::sync::Mutex::new(None),
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
            commands::list_meetings,
            commands::get_meeting,
            commands::get_jobs,
            commands::capture_sources,
            commands::save_settings,
            commands::save_key,
            commands::delete_key,
            commands::provider_models,
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
        .run(tauri::generate_context!())?;
    Ok(())
}
