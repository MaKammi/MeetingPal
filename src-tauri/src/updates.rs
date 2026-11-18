use crate::AppState;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Default)]
struct Session {
    last_attempt: Option<Instant>,
    available: Option<Update>,
}

#[derive(Default)]
pub struct Updates {
    session: Mutex<Session>,
    activity: RwLock<()>,
}

impl Updates {
    /// Hold while starting a recording, importing, or processing a user's files.
    pub fn working(&self) -> Result<RwLockReadGuard<'_, ()>, String> {
        self.activity
            .try_read()
            .map_err(|_| "UPDATE_INSTALLING".into())
    }
}

#[tauri::command]
pub async fn check_for_update(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    automatic: bool,
) -> Result<Option<String>, String> {
    let settings = state.store.settings();
    if automatic
        && !settings
            .update_checks
            .automatic(settings.default_config.offline)
    {
        return Ok(None);
    }
    if state.recording.lock().await.is_some() {
        return Err("UPDATE_RECORDING".into());
    }
    let mut session = state
        .updates
        .session
        .try_lock()
        .map_err(|_| "UPDATE_BUSY")?;
    if automatic
        && session
            .last_attempt
            .is_some_and(|last| last.elapsed() < DAY)
    {
        return Ok(session.available.as_ref().map(|u| u.version.clone()));
    }
    session.last_attempt = Some(Instant::now());
    let updater = app.updater().map_err(|e| e.to_string())?;
    let mut available = tokio::time::timeout(Duration::from_secs(25), updater.check())
        .await
        .map_err(|_| "UPDATE_TIMEOUT")?
        .map_err(|e| e.to_string())?;
    if let Some(update) = &mut available {
        // Only the release assets on our fixed HTTPS origin can ever be downloaded.
        let prefix = format!(
            "https://github.com/MaKammi/MeetingPal/releases/download/v{}/",
            update.version
        );
        if !update.download_url.as_str().starts_with(&prefix) {
            return Err("UPDATE_SOURCE".into());
        }
        update.timeout = Some(Duration::from_secs(20 * 60));
    }
    let version = available.as_ref().map(|u| u.version.clone());
    session.available = available;
    Ok(version)
}

async fn ready_to_install(state: &AppState) -> Result<(), String> {
    if state.recording.lock().await.is_some() {
        return Err("UPDATE_RECORDING".into());
    }
    if state.store.has_active_jobs().map_err(|e| e.to_string())? {
        return Err("UPDATE_WORKING".into());
    }
    Ok(())
}

#[tauri::command]
pub async fn install_update(app: AppHandle, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let session = state
        .updates
        .session
        .try_lock()
        .map_err(|_| "UPDATE_BUSY")?;
    let update = session
        .available
        .as_ref()
        .ok_or("UPDATE_CHECK_FIRST")?
        .clone();
    ready_to_install(&state).await?;
    let mut received = 0_u64;
    let mut last_percent = None;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                if let Some(total) = total.filter(|n| *n > 0) {
                    let percent = (received.saturating_mul(100) / total).min(100);
                    if last_percent != Some(percent) {
                        let _ = app.emit("update-progress", percent);
                        last_percent = Some(percent);
                    }
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    // A meeting may have started during the download. Only reserve the short
    // installation phase after the signed package has been completely verified.
    let _installing = state
        .updates
        .activity
        .try_write()
        .map_err(|_| "UPDATE_WORKING")?;
    ready_to_install(&state).await?;
    state.ai.shutdown().await;
    tokio::task::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    app.restart()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Settings, UpdateChecks};

    #[test]
    fn migrating_an_offline_profile_keeps_background_requests_disabled() {
        let mut old = serde_json::to_value(Settings::default()).unwrap();
        old.as_object_mut().unwrap().remove("updateChecks");
        let settings: Settings = serde_json::from_value(old).unwrap();
        assert!(!settings
            .update_checks
            .automatic(settings.default_config.offline));
        assert!(settings.update_checks.automatic(false));
        assert!(UpdateChecks::Always.automatic(true));
        assert!(!UpdateChecks::Manual.automatic(false));
    }

    #[test]
    fn meeting_work_and_installation_are_mutually_exclusive() {
        let updates = Updates::default();
        let recording = updates.working().unwrap();
        assert!(updates.activity.try_write().is_err());
        drop(recording);
        let _install = updates.activity.try_write().unwrap();
        assert!(updates.working().is_err());
    }
}
