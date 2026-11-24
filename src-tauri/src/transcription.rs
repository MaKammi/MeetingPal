use crate::domain::{MediaSegment, TranscriptSegment};
use anyhow::{ensure, Context, Result};
use futures_util::StreamExt;
use serde_json::Value;
use std::sync::LazyLock;

#[derive(serde::Deserialize)]
struct Model {
    provider: String,
    model: String,
    features: Vec<String>,
}
pub fn model_supports(provider: &str, model: &str, feature: &str) -> bool {
    static CATALOG: LazyLock<Vec<Model>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../../resources/model-capabilities.json"))
            .expect("invalid bundled model catalog")
    });
    CATALOG.iter().any(|m| {
        m.provider == provider && m.model == model && m.features.iter().any(|f| f == feature)
    })
}

pub fn segments(
    raw: &Value,
    source: &MediaSegment,
    provisional: bool,
    timed: bool,
    diarized: bool,
) -> Result<Vec<TranscriptSegment>> {
    let make = |i, start_ms, end_ms, text: &str, speaker| TranscriptSegment {
        id: format!("{}-{i}", source.id),
        start_ms,
        end_ms,
        text: text.trim().into(),
        speaker,
        provisional,
        edited: false,
    };
    if !timed {
        let text = raw["text"].as_str().context("Missing transcription text")?;
        return Ok(if text.trim().is_empty() {
            vec![]
        } else {
            vec![make(0, source.start_ms, source.end_ms, text, None)]
        });
    }
    let mut result = Vec::new();
    for (i, item) in raw["segments"]
        .as_array()
        .context("Missing timed transcript segments")?
        .iter()
        .enumerate()
    {
        let text = item["text"].as_str().context("Missing segment text")?;
        if text.trim().is_empty() {
            continue;
        }
        let time = |name: &str| -> Result<u64> {
            let seconds = item[name].as_f64().context("Missing segment timestamp")?;
            ensure!(
                seconds.is_finite() && seconds >= 0.0,
                "Invalid segment timestamp"
            );
            Ok(source
                .start_ms
                .saturating_add((seconds * 1000.0).round() as u64)
                .min(source.end_ms))
        };
        let start = time("start")?;
        let end = time("end")?;
        ensure!(end >= start, "Reversed transcript segment");
        // A provider's speaker IDs apply only to one request. Keep names distinct across sections.
        let speaker = if diarized {
            let label = item["speaker"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .context("Missing speaker label")?;
            let seconds = source.start_ms / 1000;
            Some(format!("{label} · {}:{:02}", seconds / 60, seconds % 60))
        } else {
            None
        };
        result.push(make(i, start, end, text, speaker));
    }
    ensure!(
        !result.is_empty() || raw["text"].as_str().unwrap_or("").trim().is_empty(),
        "Missing transcript segments for nonempty speech"
    );
    Ok(result)
}

#[derive(Default)]
struct TranscriptStream {
    pending: Vec<u8>,
    data: String,
    draft: String,
    finalized_speech: String,
    segments: Vec<Value>,
    complete: Option<String>,
}
impl TranscriptStream {
    fn feed(&mut self, bytes: &[u8], progress: &impl Fn(&str)) -> Result<()> {
        for &byte in bytes {
            if byte != b'\n' {
                ensure!(
                    self.pending.len() < 2_000_000,
                    "Transcription event is too large"
                );
                self.pending.push(byte);
                continue;
            }
            let line = std::str::from_utf8(&self.pending)?.trim_end_matches('\r');
            if line.is_empty() {
                if !self.data.is_empty() {
                    let data = std::mem::take(&mut self.data);
                    if data.trim() != "[DONE]" {
                        let event: Value = serde_json::from_str(&data)?;
                        match event["type"].as_str() {
                            Some("transcript.text.delta") => {
                                self.draft.push_str(
                                    event["delta"]
                                        .as_str()
                                        .context("Missing transcript delta")?,
                                );
                                progress(&self.draft);
                            }
                            Some("transcript.text.segment") => {
                                let text = event["text"]
                                    .as_str()
                                    .context("Missing speaker segment text")?;
                                if !self.finalized_speech.is_empty() {
                                    self.finalized_speech.push(' ');
                                }
                                self.finalized_speech.push_str(text.trim());
                                self.draft.clone_from(&self.finalized_speech);
                                progress(&self.draft);
                                self.segments.push(event);
                            }
                            Some("transcript.text.done") => {
                                self.complete = Some(
                                    event["text"]
                                        .as_str()
                                        .context("Missing final transcript")?
                                        .into(),
                                )
                            }
                            Some("error") => anyhow::bail!("Transcription stream failed"),
                            _ => {}
                        }
                    }
                }
            } else if let Some(value) = line.strip_prefix("data:") {
                ensure!(
                    self.data.len() + value.len() < 2_000_000,
                    "Transcription event is too large"
                );
                self.data.push_str(value.trim_start());
                self.data.push('\n');
            }
            self.pending.clear();
        }
        Ok(())
    }
    fn finish(self) -> Result<Value> {
        Ok(
            serde_json::json!({"text": self.complete.context("Retryable: incomplete transcription stream")?, "segments": self.segments}),
        )
    }
}

pub async fn read_stream(response: reqwest::Response, progress: impl Fn(&str)) -> Result<Value> {
    let mut stream = response.bytes_stream();
    let mut transcript = TranscriptStream::default();
    while let Some(chunk) = stream.next().await {
        transcript.feed(
            &chunk.context("Retryable: transcription stream interrupted")?,
            &progress,
        )?;
    }
    transcript.feed(b"\n\n", &progress)?;
    transcript.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_byte_boundaries_and_incomplete_streams() {
        let data = "data: {\"type\":\"transcript.text.delta\",\"delta\":\"Grüße\"}\r\n\r\ndata: {\"type\":\"transcript.text.done\",\"text\":\"Grüße\"}\n\n";
        let mut s = TranscriptStream::default();
        for b in data.as_bytes() {
            s.feed(&[*b], &|v| assert_eq!(v, "Grüße")).unwrap();
        }
        assert_eq!(s.finish().unwrap()["text"], "Grüße");
        assert!(TranscriptStream::default().finish().is_err());
    }
    #[test]
    fn diarization_offsets_and_scopes_speakers() {
        let src = MediaSegment {
            id: "s".into(),
            start_ms: 65000,
            end_ms: 70000,
            microphone: None,
            desktop: None,
            mixed: "a.wav".into(),
        };
        let input =
            serde_json::json!({"segments":[{"start":0.5,"end":9,"speaker":"A","text":" hi "}]});
        let result = segments(&input, &src, false, true, true).unwrap();
        assert_eq!((result[0].start_ms, result[0].end_ms), (65500, 70000));
        assert_eq!(result[0].speaker.as_deref(), Some("A · 1:05"));
    }
}
