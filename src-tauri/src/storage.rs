use crate::domain::*;
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};

pub struct Store {
    pub root: PathBuf,
    db: Mutex<Connection>,
    gate: Mutex<()>,
}
pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let tmp = path.with_extension("json.pending");
    let mut file = fs::File::create(&tmp)?;
    serde_json::to_writer_pretty(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    // Windows rename does not replace an existing file. Keep a recovery copy during replacement.
    let backup = path.with_extension("json.previous");
    if path.exists() {
        fs::copy(path, &backup)?;
    }
    #[cfg(target_os = "windows")]
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(&tmp, path)?;
    if backup.exists() {
        let _ = fs::remove_file(backup);
    }
    Ok(())
}
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let primary =
        fs::read(path).and_then(|b| serde_json::from_slice(&b).map_err(std::io::Error::other));
    match primary {
        Ok(value) => Ok(value),
        Err(error) => {
            for extension in ["json.pending", "json.previous"] {
                if let Ok(bytes) = fs::read(path.with_extension(extension)) {
                    if let Ok(value) = serde_json::from_slice(&bytes) {
                        return Ok(value);
                    }
                }
            }
            Err(error.into())
        }
    }
}
pub fn safe_child(root: &Path, relative: &str) -> Result<PathBuf> {
    let p = Path::new(relative);
    anyhow::ensure!(
        !relative.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_))),
        "Invalid relative path"
    );
    let joined = root.join(p);
    if joined.exists() {
        anyhow::ensure!(
            joined.canonicalize()?.starts_with(root.canonicalize()?),
            "Path leaves the meeting folder"
        );
    }
    Ok(joined)
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        fs::create_dir_all(root.join("meetings"))?;
        fs::create_dir_all(root.join("models"))?;
        let db = Connection::open(root.join("library.sqlite"))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;
            CREATE TABLE IF NOT EXISTS meetings(id TEXT PRIMARY KEY, summary TEXT NOT NULL, search TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY, meeting_id TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, state TEXT NOT NULL, attempts INTEGER NOT NULL DEFAULT 0, error TEXT, next_at INTEGER NOT NULL DEFAULT 0);
            CREATE INDEX IF NOT EXISTS jobs_pending ON jobs(state,next_at);
            UPDATE jobs SET state='pending' WHERE state='running';")?;
        let store = Self {
            root,
            db: Mutex::new(db),
            gate: Mutex::new(()),
        };
        store.rebuild_index()?;
        Ok(store)
    }
    pub fn folder(&self, id: &str) -> Result<PathBuf> {
        uuid::Uuid::parse_str(id).context("Invalid meeting identifier")?;
        safe_child(&self.root.join("meetings"), id)
    }
    pub fn load(&self, id: &str) -> Result<Meeting> {
        read_json(&self.folder(id)?.join("meeting.json"))
    }
    pub fn save(&self, meeting: &Meeting) -> Result<()> {
        let _guard = self.gate.lock().unwrap();
        self.save_unlocked(meeting)
    }
    fn save_unlocked(&self, meeting: &Meeting) -> Result<()> {
        let folder = self.folder(&meeting.id)?;
        fs::create_dir_all(&folder)?;
        atomic_json(&folder.join("meeting.json"), meeting)?;
        self.index(meeting)
    }
    fn index(&self, meeting: &Meeting) -> Result<()> {
        let search = format!(
            "{} {} {} {}",
            meeting.config.title,
            meeting.tags.join(" "),
            meeting.notes,
            meeting
                .transcript
                .iter()
                .map(|t| t.text.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        );
        self.db.lock().unwrap().execute("INSERT INTO meetings(id,summary,search) VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET summary=excluded.summary,search=excluded.search", params![meeting.id, serde_json::to_string(&MeetingSummary::from(meeting))?, search])?;
        Ok(())
    }
    pub fn update(
        &self,
        id: &str,
        action: impl FnOnce(&mut Meeting) -> Result<()>,
    ) -> Result<Meeting> {
        let _guard = self.gate.lock().unwrap();
        let mut meeting = self.load(id)?;
        action(&mut meeting)?;
        meeting.updated_at = chrono::Utc::now().to_rfc3339();
        self.save_unlocked(&meeting)?;
        Ok(meeting)
    }
    pub fn list(&self, query: &str) -> Result<Vec<MeetingSummary>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT summary FROM meetings WHERE instr(lower(search),lower(?1)) > 0 ORDER BY json_extract(summary,'$.createdAt') DESC")?;
        let rows = stmt.query_map([query], |row| row.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn rebuild_index(&self) -> Result<()> {
        for entry in fs::read_dir(self.root.join("meetings"))? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let path = entry.path().join("meeting.json");
            if let Ok(mut meeting) = read_json::<Meeting>(&path) {
                if matches!(meeting.status.as_str(), "recording" | "paused" | "starting") {
                    if let Ok(manifest) = read_json::<crate::capture::CaptureManifest>(
                        &entry.path().join("capture.json"),
                    ) {
                        meeting.media = manifest.segments;
                        meeting.duration_ms = manifest.duration_ms;
                        meeting.video_files = manifest.video_files;
                    }
                    meeting.status = "recovered".into();
                    meeting.warnings.push("Recording interrupted. Completed segments were recovered; the final unfinished segment may be missing.".into());
                    atomic_json(&path, &meeting)?;
                }
                self.index(&meeting)?;
            }
        }
        Ok(())
    }
    pub fn enqueue(&self, job: &Job) -> Result<()> {
        self.db.lock().unwrap().execute("INSERT OR IGNORE INTO jobs(id,meeting_id,kind,payload,state) VALUES(?1,?2,?3,?4,'pending')", params![job.id, job.meeting_id, job.kind, job.payload])?;
        Ok(())
    }
    pub fn next_job(&self) -> Result<Option<Job>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT id,meeting_id,kind,payload,state,attempts,error FROM jobs WHERE state='pending' AND next_at <= unixepoch() ORDER BY rowid LIMIT 1")?;
        let job = stmt.query_map([], job_row)?.next().transpose()?;
        if let Some(job) = &job {
            db.execute(
                "UPDATE jobs SET state='running',attempts=attempts+1 WHERE id=?1",
                [&job.id],
            )?;
        }
        Ok(job)
    }
    pub fn finish_job(&self, id: &str, error: Option<&str>, retry: bool) -> Result<()> {
        let state = if error.is_none() {
            "done"
        } else if retry {
            "pending"
        } else {
            "failed"
        };
        self.db.lock().unwrap().execute(
            "UPDATE jobs SET state=?2,error=?3,next_at=unixepoch()+min(60,5*attempts) WHERE id=?1",
            params![id, state, error],
        )?;
        Ok(())
    }
    pub fn jobs(&self, meeting_id: &str) -> Result<Vec<Job>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT id,meeting_id,kind,payload,state,attempts,error FROM jobs WHERE meeting_id=?1 ORDER BY rowid")?;
        let rows = stmt.query_map([meeting_id], job_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn retry_jobs(&self, id: &str) -> Result<()> {
        self.db.lock().unwrap().execute("UPDATE jobs SET state='pending',attempts=0,error=NULL,next_at=0 WHERE meeting_id=?1 AND state='failed'", [id])?;
        Ok(())
    }
    pub fn defer_job(&self, id: &str) -> Result<()> {
        self.db.lock().unwrap().execute("UPDATE jobs SET state='pending',attempts=max(0,attempts-1),next_at=unixepoch()+2 WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn has_active_jobs(&self) -> Result<bool> {
        Ok(self.db.lock().unwrap().query_row(
            "SELECT EXISTS(SELECT 1 FROM jobs WHERE state IN ('pending','running'))",
            [],
            |r| r.get(0),
        )?)
    }
    pub fn archive_analysis(&self, id: &str, analysis: &AnalysisResult) -> Result<()> {
        let folder = self.folder(id)?.join("results");
        fs::create_dir_all(&folder)?;
        atomic_json(
            &folder.join(format!("analysis-{}.json", uuid::Uuid::new_v4())),
            analysis,
        )
    }
    pub fn delete(&self, id: &str, media_only: bool) -> Result<()> {
        let folder = self.folder(id)?;
        if media_only {
            self.update(id, |m| {
                for entry in fs::read_dir(&folder)? {
                    let entry = entry?;
                    let p = entry.path();
                    if entry.file_type()?.is_file()
                        && matches!(
                            p.extension().and_then(|s| s.to_str()),
                            Some("wav" | "webm" | "mp4" | "mkv" | "jpg" | "png")
                        )
                    {
                        fs::remove_file(p)?;
                    }
                }
                m.media.clear();
                m.video_files.clear();
                m.playback_file = None;
                Ok(())
            })?;
        } else {
            if folder.exists() {
                fs::remove_dir_all(folder)?;
            }
            let db = self.db.lock().unwrap();
            db.execute("DELETE FROM meetings WHERE id=?1", [id])?;
            db.execute("DELETE FROM jobs WHERE meeting_id=?1", [id])?;
        }
        Ok(())
    }
    pub fn settings(&self) -> Settings {
        read_json(&self.root.join("settings.json")).unwrap_or_default()
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        let _guard = self.gate.lock().unwrap();
        atomic_json(&self.root.join("settings.json"), settings)
    }
}
fn job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: row.get(0)?,
        meeting_id: row.get(1)?,
        kind: row.get(2)?,
        payload: row.get(3)?,
        state: row.get(4)?,
        attempts: row.get(5)?,
        error: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_cannot_escape() {
        let d = tempfile::tempdir().unwrap();
        assert!(safe_child(d.path(), "../../key").is_err());
        assert!(safe_child(d.path(), "C:\\secrets").is_err() || !cfg!(windows));
    }
    #[test]
    fn queue_is_durable_and_idempotent() {
        let d = tempfile::tempdir().unwrap();
        let store = Store::open(d.path().to_owned()).unwrap();
        let m = Meeting::new(MeetingConfig::default());
        store.save(&m).unwrap();
        let j = Job {
            id: "a".into(),
            meeting_id: m.id.clone(),
            kind: "transcribe".into(),
            payload: "0".into(),
            state: "pending".into(),
            attempts: 0,
            error: None,
        };
        store.enqueue(&j).unwrap();
        store.enqueue(&j).unwrap();
        assert_eq!(store.jobs(&m.id).unwrap().len(), 1);
        assert!(store.next_job().unwrap().is_some());
        assert!(store.next_job().unwrap().is_none());
        drop(store);
        let reopened = Store::open(d.path().to_owned()).unwrap();
        assert!(reopened.next_job().unwrap().is_some());
    }
    #[test]
    fn interrupted_recording_recovers_only_committed_segments() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path().to_owned()).unwrap();
        let mut meeting = Meeting::new(MeetingConfig::default());
        meeting.status = "recording".into();
        store.save(&meeting).unwrap();
        let folder = store.folder(&meeting.id).unwrap();
        let segment = MediaSegment {
            id: "audio-000000".into(),
            start_ms: 0,
            end_ms: 5000,
            microphone: Some("mic.wav".into()),
            desktop: Some("desktop.wav".into()),
            mixed: "mix.wav".into(),
        };
        atomic_json(
            &folder.join("capture.json"),
            &crate::capture::CaptureManifest {
                segments: vec![segment],
                duration_ms: 5000,
                video_files: vec![],
            },
        )
        .unwrap();
        fs::write(folder.join("mixed-incomplete.wav"), b"incomplete").unwrap();
        drop(store);
        let reopened = Store::open(dir.path().to_owned()).unwrap();
        let recovered = reopened.load(&meeting.id).unwrap();
        assert_eq!(recovered.status, "recovered");
        assert_eq!(recovered.duration_ms, 5000);
        assert_eq!(recovered.media.len(), 1);
    }
    #[test]
    fn recovers_json_when_replacement_is_interrupted() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("meeting.json");
        fs::write(path.with_extension("json.pending"), br#"{"answer":42}"#).unwrap();
        let result: serde_json::Value = read_json(&path).unwrap();
        assert_eq!(result["answer"], 42);
    }
}
