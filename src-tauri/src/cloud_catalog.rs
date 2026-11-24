use crate::{domain::Provider, providers::checked, secrets::Secrets};
use anyhow::{ensure, Context, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ListedModel {
    pub id: String,
    pub inputs: Vec<&'static str>,
    #[serde(skip)]
    created: i64,
}

fn listed(provider: &Provider, item: &Value) -> Option<ListedModel> {
    let id = item["id"]
        .as_str()
        .or(item["name"].as_str())?
        .trim_start_matches("models/");
    if id.is_empty()
        || item["lifecycle"] == "retired"
        || item["shutdown_date"]
            .as_str()
            .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
            .is_some_and(|day| day <= chrono::Utc::now().date_naive())
    {
        return None;
    }
    let specialist = |words: &[&str]| id.split('-').any(|part| words.contains(&part));
    let mut inputs = match provider {
        Provider::Gemini => {
            if !item["supportedGenerationMethods"]
                .as_array()?
                .iter()
                .any(|m| m == "generateContent")
                || !id.starts_with("gemini-")
                || (item["outputModalities"].as_array().is_none()
                    && specialist(&[
                        "tts",
                        "image",
                        "embedding",
                        "transcribe",
                        "live",
                        "audio",
                        "robotics",
                        "computer",
                        "banana",
                        "decision",
                        "omni",
                        "customtools",
                    ]))
            {
                return None;
            }
            vec!["text", "image", "video"]
        }
        Provider::Anthropic => {
            let mut inputs = vec!["text"];
            if item["capabilities"]["image_input"]["supported"]
                .as_bool()
                .unwrap_or(true)
            {
                inputs.push("image");
            }
            inputs
        }
        Provider::Openai => {
            if !specialist(&["live", "realtime"])
                && (specialist(&["transcribe"]) || id == "whisper-1")
            {
                vec!["audio"]
            } else if (id.starts_with("gpt-")
                || id == "chat-latest"
                || id
                    .strip_prefix('o')
                    .is_some_and(|s| s.starts_with(|c: char| c.is_ascii_digit())))
                && !specialist(&[
                    "live",
                    "realtime",
                    "transcribe",
                    "tts",
                    "audio",
                    "image",
                    "search",
                    "codex",
                    "cyber",
                    "deep",
                    "research",
                    "rosalind",
                    "computer",
                    "instruct",
                ])
            {
                if id.starts_with("gpt-3")
                    || id == "gpt-4"
                    || id.starts_with("o1-mini")
                    || id.starts_with("o3-mini")
                {
                    vec!["text"]
                } else {
                    vec!["text", "image"]
                }
            } else {
                return None;
            }
        }
        Provider::Local => return None,
    };
    if let Some(output) = item["outputModalities"].as_array() {
        if provider != &Provider::Openai
            && !output
                .iter()
                .any(|v| v.as_str().is_some_and(|s| s.eq_ignore_ascii_case("text")))
        {
            return None;
        }
    }
    if let Some(reported) = item["inputModalities"].as_array() {
        inputs.retain(|task| {
            reported
                .iter()
                .any(|v| v.as_str().is_some_and(|s| s.eq_ignore_ascii_case(task)))
        });
    }
    let created = item["created"]
        .as_i64()
        .or_else(|| {
            item["created_at"]
                .as_str()
                .or(item["createTime"].as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.timestamp())
        })
        .unwrap_or(0);
    (!inputs.is_empty()).then(|| ListedModel {
        id: id.to_owned(),
        inputs,
        created,
    })
}

fn gemini_generation(id: &str) -> Vec<u32> {
    id.strip_prefix("gemini-")
        .unwrap_or(id)
        .split('-')
        .next()
        .unwrap_or("")
        .split('.')
        .map_while(|part| part.parse().ok())
        .collect()
}

fn sort_models(provider: &Provider, models: &mut [ListedModel]) {
    models.sort_by(|a, b| {
        let by_date = b.created.cmp(&a.created);
        if by_date.is_ne() {
            return by_date;
        }
        if *provider == Provider::Gemini {
            // Google lists oldest first and currently omits release dates. Compare numerical generations,
            // then prefer a stable model within that generation. Preserve vendor order for peers.
            return gemini_generation(&b.id)
                .cmp(&gemini_generation(&a.id))
                .then_with(|| a.id.contains("preview").cmp(&b.id.contains("preview")));
        }
        a.id.cmp(&b.id)
    });
}

pub async fn load(
    client: &reqwest::Client,
    secrets: &Secrets,
    provider: &Provider,
) -> Result<Vec<ListedModel>> {
    ensure!(*provider != Provider::Local, "Choose a cloud provider");
    let key = secrets.get(provider)?;
    let mut result = Vec::new();
    let mut ids = HashSet::new();
    let mut cursors = HashSet::new();
    let mut cursor = None;
    loop {
        let mut request = match provider {
            Provider::Gemini => client
                .get("https://generativelanguage.googleapis.com/v1beta/models")
                .header("x-goog-api-key", &key)
                .query(&[("pageSize", "1000")]),
            Provider::Anthropic => client
                .get("https://api.anthropic.com/v1/models")
                .header("x-api-key", &key)
                .header("anthropic-version", "2023-06-01")
                .query(&[("limit", "100")]),
            Provider::Openai => client
                .get("https://api.openai.com/v1/models")
                .bearer_auth(&key),
            Provider::Local => anyhow::bail!("Choose a cloud provider"),
        };
        if let Some(token) = &cursor {
            request = request.query(&[(
                if *provider == Provider::Gemini {
                    "pageToken"
                } else {
                    "after_id"
                },
                token,
            )]);
        }
        let data: Value = checked(request.send().await?)?.json().await?;
        let items = data[if *provider == Provider::Gemini {
            "models"
        } else {
            "data"
        }]
        .as_array()
        .context("No model list returned")?;
        for item in items {
            if let Some(model) = listed(provider, item) {
                if ids.insert(model.id.clone()) {
                    result.push(model);
                }
            }
        }
        let next = match provider {
            Provider::Gemini => data["nextPageToken"].as_str().filter(|t| !t.is_empty()),
            Provider::Anthropic if data["has_more"] == true => Some(
                data["last_id"]
                    .as_str()
                    .context("Missing model-list cursor")?,
            ),
            _ => None,
        };
        let Some(next) = next else { break };
        ensure!(
            cursors.insert(next.to_owned()),
            "Provider repeated a model-list cursor"
        );
        cursor = Some(next.to_owned());
    }
    sort_models(provider, &mut result);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn future_generations_and_tasks_follow_vendor_metadata() {
        let data = |id: &str| json!({"name":format!("models/{id}"),"supportedGenerationMethods":["generateContent"]});
        let mut models: Vec<_> = [
            "gemini-3.9-flash",
            "gemini-4-new-preview",
            "gemini-3.10-pro",
            "gemini-4-new",
            "gemini-4-new-tts",
        ]
        .iter()
        .filter_map(|id| listed(&Provider::Gemini, &data(id)))
        .collect();
        sort_models(&Provider::Gemini, &mut models);
        assert_eq!(
            models.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
            [
                "gemini-4-new",
                "gemini-4-new-preview",
                "gemini-3.10-pro",
                "gemini-3.9-flash"
            ]
        );
        let mut text = data("gemini-4-new");
        text["inputModalities"] = json!(["TEXT", "IMAGE"]);
        assert!(!listed(&Provider::Gemini, &text)
            .unwrap()
            .inputs
            .contains(&"video"));
        assert!(listed(&Provider::Openai, &json!({"id":"gpt-live-transcribe"})).is_none());
        assert_eq!(
            listed(&Provider::Openai, &json!({"id":"gpt-transcribe"}))
                .unwrap()
                .inputs,
            ["audio"]
        );
        assert_eq!(
            listed(
                &Provider::Anthropic,
                &json!({"id":"claude-new","capabilities":{"image_input":{"supported":false}}})
            )
            .unwrap()
            .inputs,
            ["text"]
        );
    }
}
