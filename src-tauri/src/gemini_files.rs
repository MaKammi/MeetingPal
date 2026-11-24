use crate::{
    domain::Provider,
    providers::{checked, reserve_cloud, reserve_cloud_requests},
    storage::Store,
};
use anyhow::{ensure, Context, Result};
use reqwest::{Client, Url};
use serde::Deserialize;
use std::{path::Path, time::Duration};

const BASE: &str = "https://generativelanguage.googleapis.com";
#[derive(Deserialize)]
struct FileInfo {
    name: String,
    state: String,
}

pub(crate) struct UploadedFile {
    pub uri: String,
    state: String,
}
impl UploadedFile {
    pub async fn wait_until_ready(
        &self,
        client: &Client,
        store: &Store,
        meeting: &str,
        key: &str,
    ) -> Result<()> {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(240);
        let mut state = self.state.clone();
        while state == "PROCESSING" {
            ensure!(
                tokio::time::Instant::now() < deadline,
                "Retryable: Gemini video processing timed out"
            );
            tokio::time::sleep(Duration::from_secs(3)).await;
            reserve_cloud(store, meeting, &Provider::Gemini)?;
            let info: FileInfo = checked(
                client
                    .get(&self.uri)
                    .header("x-goog-api-key", key)
                    .send()
                    .await?,
            )?
            .json()
            .await?;
            state = info.state;
        }
        ensure!(state == "ACTIVE", "Gemini could not process the video");
        Ok(())
    }
    pub async fn delete(&self, client: &Client, key: &str) -> Result<()> {
        let response = client
            .delete(&self.uri)
            .header("x-goog-api-key", key)
            .send()
            .await?;
        if response.status() != reqwest::StatusCode::NOT_FOUND {
            checked(response)?;
        }
        Ok(())
    }
}
pub(crate) async fn upload(
    client: &Client,
    store: &Store,
    meeting: &str,
    key: &str,
    path: &Path,
) -> Result<UploadedFile> {
    let file = tokio::fs::File::open(path).await?;
    let size = file.metadata().await?.len();
    ensure!(
        (1..=2_000_000_000).contains(&size),
        "Gemini video part must be between 1 byte and 2 GB"
    );
    // Reserve session creation, file transfer and deletion before sending any meeting bytes.
    reserve_cloud_requests(store, meeting, &Provider::Gemini, 3)?;
    let start = checked(
        client
            .post(format!("{BASE}/upload/v1beta/files"))
            .header("x-goog-api-key", key)
            .header("x-goog-upload-protocol", "resumable")
            .header("x-goog-upload-command", "start")
            .header("x-goog-upload-header-content-length", size)
            .header("x-goog-upload-header-content-type", "video/webm")
            .json(&serde_json::json!({"file":{"display_name":"MeetingPal clip"}}))
            .send()
            .await?,
    )?;
    let session = Url::parse(
        start
            .headers()
            .get("x-goog-upload-url")
            .context("Gemini omitted upload session")?
            .to_str()?,
    )?;
    ensure!(
        session.scheme() == "https"
            && session
                .host_str()
                .is_some_and(|h| h.ends_with(".googleapis.com")),
        "Invalid Google upload endpoint"
    );
    let finish = client
        .post(session)
        .header("content-length", size)
        .header("x-goog-upload-offset", "0")
        .header("x-goog-upload-command", "upload, finalize")
        .timeout(Duration::from_secs(900))
        .body(file)
        .send()
        .await
        .map_err(reqwest::Error::without_url)?;
    #[derive(Deserialize)]
    struct Upload {
        file: FileInfo,
    }
    let upload: Upload = checked(finish)?
        .json()
        .await
        .map_err(reqwest::Error::without_url)?;
    let id = upload
        .file
        .name
        .strip_prefix("files/")
        .context("Invalid Gemini file name")?;
    ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "Invalid Gemini file ID"
    );
    Ok(UploadedFile {
        uri: format!("{BASE}/v1beta/files/{id}"),
        state: upload.file.state,
    })
}
