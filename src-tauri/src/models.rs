use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::Emitter;
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelFile {
    pub filename: String,
    pub url: String,
    pub sha256: String,
    pub bytes: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    pub task: String,
    pub license: String,
    pub source: String,
    pub files: Vec<ModelFile>,
    #[serde(default)]
    pub installed: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub id: String,
    pub received: u64,
    pub total: u64,
    pub state: String,
    pub error: Option<String>,
}

pub struct Models {
    pub root: PathBuf,
    active: Mutex<HashSet<String>>,
    cancelled: Mutex<std::collections::HashMap<String, Arc<AtomicBool>>>,
}
impl Models {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            active: Mutex::new(HashSet::new()),
            cancelled: Mutex::new(Default::default()),
        }
    }
    pub fn catalog(&self) -> Vec<ModelSpec> {
        let mut specs: Vec<ModelSpec> =
            serde_json::from_str(include_str!("../../resources/models.json"))
                .expect("validated model catalog");
        for spec in &mut specs {
            let verified: Option<ModelSpec> =
                crate::storage::read_json(&self.root.join(&spec.id).join("verified.json")).ok();
            spec.installed = verified.is_some_and(|v| {
                v.files.len() == spec.files.len()
                    && v.files.iter().zip(&spec.files).all(|(a, b)| {
                        a.filename == b.filename && a.sha256 == b.sha256 && a.bytes == b.bytes
                    })
            }) && spec.files.iter().all(|f| {
                fs::metadata(self.root.join(&spec.id).join(&f.filename))
                    .is_ok_and(|m| m.len() == f.bytes)
            });
        }
        specs
    }
    pub fn spec(&self, id: &str) -> Result<ModelSpec> {
        self.catalog()
            .into_iter()
            .find(|s| s.id == id)
            .context("Unknown model")
    }
    pub fn path(&self, id: &str) -> Result<PathBuf> {
        let spec = self.spec(id)?;
        anyhow::ensure!(
            spec.installed,
            "Model {} is not installed. Download it in Models.",
            spec.name
        );
        Ok(self.root.join(id).join(&spec.files[0].filename))
    }
    pub fn projector(&self, id: &str) -> Result<Option<PathBuf>> {
        let spec = self.spec(id)?;
        Ok(spec
            .files
            .iter()
            .find(|f| f.filename.contains("mmproj"))
            .map(|f| self.root.join(id).join(&f.filename)))
    }
    pub fn cancel(&self, id: &str) {
        if let Some(cancel) = self.cancelled.lock().unwrap().get(id) {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn remove(&self, id: &str) -> Result<()> {
        self.spec(id)?;
        anyhow::ensure!(
            !self.active.lock().unwrap().contains(id),
            "Cancel the download first."
        );
        let folder = self.root.join(id);
        if folder.exists() {
            fs::remove_dir_all(folder)?;
        }
        Ok(())
    }
    pub async fn download(&self, id: String, app: &tauri::AppHandle) -> Result<()> {
        self.download_with_progress(id, |progress| {
            let _ = app.emit("model-download", progress);
        })
        .await
    }
    pub async fn download_with_progress(
        &self,
        id: String,
        progress: impl Fn(DownloadProgress) + Send + Sync,
    ) -> Result<()> {
        let spec = self.spec(&id)?;
        anyhow::ensure!(
            self.active.lock().unwrap().insert(id.clone()),
            "This model is already downloading."
        );
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancelled
            .lock()
            .unwrap()
            .insert(id.clone(), cancel.clone());
        let result = self.download_inner(&spec, &progress, cancel).await;
        self.active.lock().unwrap().remove(&id);
        self.cancelled.lock().unwrap().remove(&id);
        progress(DownloadProgress {
            id,
            received: 0,
            total: spec.files.iter().map(|f| f.bytes).sum(),
            state: if result.is_ok() { "complete" } else { "failed" }.into(),
            error: result.as_ref().err().map(|e| e.to_string()),
        });
        result
    }
    async fn download_inner(
        &self,
        spec: &ModelSpec,
        progress: &(impl Fn(DownloadProgress) + Send + Sync),
        cancel: Arc<AtomicBool>,
    ) -> Result<()> {
        let folder = self.root.join(&spec.id);
        fs::create_dir_all(&folder)?;
        let total: u64 = spec.files.iter().map(|f| f.bytes).sum();
        anyhow::ensure!(
            fs2::available_space(&folder)? > total + 200_000_000,
            "Not enough disk space for this model."
        );
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()?;
        let mut received = 0u64;
        for file in &spec.files {
            anyhow::ensure!(
                file.url.starts_with("https://huggingface.co/") && file.sha256.len() == 64,
                "Invalid model manifest"
            );
            let final_path = folder.join(&file.filename);
            if final_path.exists() && verify(&final_path, &file.sha256)? {
                received += file.bytes;
                continue;
            }
            let partial = final_path.with_extension("download");
            let response = client.get(&file.url).send().await?.error_for_status()?;
            let mut stream = response.bytes_stream();
            let mut output = tokio::fs::File::create(&partial).await?;
            let mut digest = Sha256::new();
            let mut file_size = 0u64;
            let mut last = std::time::Instant::now();
            while let Some(bytes) = stream.next().await {
                anyhow::ensure!(
                    !cancel.load(Ordering::Relaxed),
                    "Download cancelled. Partial files are never used as models."
                );
                let bytes = bytes?;
                file_size += bytes.len() as u64;
                anyhow::ensure!(
                    file_size <= file.bytes,
                    "Download exceeded the catalog size."
                );
                digest.update(&bytes);
                output.write_all(&bytes).await?;
                if last.elapsed().as_millis() > 200 {
                    progress(DownloadProgress {
                        id: spec.id.clone(),
                        received: received + file_size,
                        total,
                        state: "downloading".into(),
                        error: None,
                    });
                    last = std::time::Instant::now();
                }
            }
            output.sync_all().await?;
            drop(output);
            anyhow::ensure!(
                file_size == file.bytes && hex::encode(digest.finalize()) == file.sha256,
                "Model checksum mismatch. The download will not be loaded."
            );
            if final_path.exists() {
                fs::remove_file(&final_path)?;
            }
            fs::rename(partial, final_path)?;
            received += file_size;
        }
        crate::storage::atomic_json(&folder.join("verified.json"), spec)?;
        Ok(())
    }
}
pub fn verify(path: &Path, expected: &str) -> Result<bool> {
    let mut file = fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut buffer = vec![0; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    Ok(hex::encode(digest.finalize()) == expected)
}
