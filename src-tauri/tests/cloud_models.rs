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
#[ignore = "discovers live models and probes four provider selections with public or synthetic inputs"]
async fn newest_available_models() -> Result<()> {
    let client = reqwest::Client::new();
    let secrets = Secrets::default();
    let mut failures = Vec::new();
    for (provider, tasks) in [
        (Provider::Gemini, vec![("video", Task::Video)]),
        (Provider::Anthropic, vec![("text", Task::Text)]),
        (
            Provider::Openai,
            vec![("audio", Task::Audio), ("text", Task::Text)],
        ),
    ] {
        let models = meetingpal_lib::cloud_catalog::load(&client, &secrets, &provider).await?;
        for (input, task) in tasks {
            let model = models
                .iter()
                .find(|m| m.inputs.contains(&input))
                .expect("Provider supplied no applicable model");
            println!("{} {input}: {}", provider.key(), model.id);
            if let Err(error) = probe(provider.clone(), &model.id, task).await {
                failures.push(format!(
                    "{} {input} {}: {error:#}",
                    provider.key(),
                    model.id
                ));
            }
        }
    }
    anyhow::ensure!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[tokio::test]
#[ignore = "calls the dedicated Gemini speech model with public test audio"]
async fn gemini_audio_dedicated() -> Result<()> {
    probe(Provider::Gemini, "gemini-3.5-transcribe", Task::Audio).await
}
#[tokio::test]
#[ignore = "discovers the newest available Gemini audio model and transcribes public test audio"]
async fn gemini_audio_latest() -> Result<()> {
    let models = meetingpal_lib::cloud_catalog::load(
        &reqwest::Client::new(),
        &Secrets::default(),
        &Provider::Gemini,
    )
    .await?;
    let latest = models
        .iter()
        .find(|model| model.inputs.contains(&"audio"))
        .expect("No Gemini audio model");
    probe(Provider::Gemini, &latest.id, Task::Audio).await
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
