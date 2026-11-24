use anyhow::Result;
use meetingpal_lib::{
    domain::{ModelSelection, Provider},
    media::Runtimes,
    model_probe::{self, Task},
    secrets::Secrets,
};
use std::path::PathBuf;

async fn probe(provider: Provider, model: &str, task: Task) -> Result<()> {
    let report = model_probe::run(
        Secrets::default(),
        Runtimes {
            directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("binaries"),
        },
        ModelSelection {
            provider,
            model: model.into(),
        },
        task,
    )
    .await?;
    println!(
        "{} {:?}: {} ms, {} requests",
        report.model, report.task, report.elapsed_ms, report.requests
    );
    Ok(())
}
#[tokio::test]
#[ignore = "calls a paid provider using the configured MeetingPal key and public test audio"]
async fn openai_speaker_stream() -> Result<()> {
    probe(Provider::Openai, "gpt-4o-transcribe-diarize", Task::Audio).await
}
#[tokio::test]
#[ignore = "calls a paid provider using the configured MeetingPal key and public test audio"]
async fn openai_word_stream() -> Result<()> {
    probe(Provider::Openai, "gpt-4o-mini-transcribe", Task::Audio).await
}
#[tokio::test]
#[ignore = "calls a paid provider with a synthetic meeting"]
async fn openai_text() -> Result<()> {
    probe(Provider::Openai, "gpt-4.1-mini", Task::Text).await
}
#[tokio::test]
#[ignore = "calls a paid provider with a synthetic meeting"]
async fn anthropic_text() -> Result<()> {
    probe(Provider::Anthropic, "claude-haiku-4-5", Task::Text).await
}
#[tokio::test]
#[ignore = "uploads a synthetic video with the configured MeetingPal key"]
async fn gemini_video() -> Result<()> {
    probe(Provider::Gemini, "gemini-2.5-flash", Task::Video).await
}
