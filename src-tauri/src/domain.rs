use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const SEGMENT_MS: u64 = 5_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Post,
    Live,
    Assistant,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Local,
    Openai,
    Anthropic,
    Gemini,
}
impl Provider {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Openai => "openai",
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelSelection {
    pub provider: Provider,
    pub model: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoKind {
    None,
    Screen,
    Window,
    Region,
    Portal,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoSelection {
    pub kind: VideoKind,
    pub id: Option<String>,
    pub crop: Option<Crop>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoAnalysis {
    None,
    Frames,
    Native,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingConfig {
    pub title: String,
    pub language: String,
    pub mode: Mode,
    pub offline: bool,
    pub microphone: Option<String>,
    pub system_device: Option<String>,
    pub video: VideoSelection,
    pub transcription: ModelSelection,
    pub analysis: ModelSelection,
    pub vision: ModelSelection,
    pub video_analysis: VideoAnalysis,
    pub max_requests: u32,
    pub consent: bool,
    pub denoise: bool,
}
impl Default for MeetingConfig {
    fn default() -> Self {
        Self {
            title: String::new(),
            language: "auto".into(),
            mode: Mode::Post,
            offline: true,
            microphone: None,
            system_device: None,
            video: VideoSelection {
                kind: VideoKind::None,
                id: None,
                crop: None,
            },
            transcription: ModelSelection {
                provider: Provider::Local,
                model: "whisper-small".into(),
            },
            analysis: ModelSelection {
                provider: Provider::Local,
                model: "qwen-text".into(),
            },
            vision: ModelSelection {
                provider: Provider::Local,
                model: "qwen-vision".into(),
            },
            video_analysis: VideoAnalysis::None,
            max_requests: 2000,
            consent: false,
            denoise: true,
        }
    }
}
impl MeetingConfig {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.consent,
            "Please confirm that recording consent has been obtained."
        );
        anyhow::ensure!(
            !self.title.trim().is_empty() && self.title.len() <= 250,
            "Enter a meeting title (maximum 250 characters)."
        );
        anyhow::ensure!(
            matches!(
                self.language.as_str(),
                "auto" | "de" | "en" | "fr" | "es" | "it" | "pt" | "nl" | "ja" | "zh"
            ),
            "Unsupported language"
        );
        anyhow::ensure!(
            (1..=100_000).contains(&self.max_requests),
            "Request budget must be between 1 and 100000."
        );
        let selections = [&self.transcription, &self.analysis, &self.vision];
        for selection in selections {
            anyhow::ensure!(
                !selection.model.trim().is_empty() && selection.model.len() <= 200,
                "Select a model for each task."
            );
        }
        if self.offline {
            anyhow::ensure!(
                self.transcription.provider == Provider::Local
                    && self.analysis.provider == Provider::Local
                    && (self.video_analysis == VideoAnalysis::None
                        || self.vision.provider == Provider::Local),
                "Offline mode requires local models for every enabled task."
            );
        }
        anyhow::ensure!(
            matches!(
                self.transcription.provider,
                Provider::Local | Provider::Openai
            ),
            "Transcription requires a local Whisper model or OpenAI."
        );
        if self.video_analysis == VideoAnalysis::Native {
            anyhow::ensure!(
                self.vision.provider == Provider::Gemini && !self.offline,
                "Native video analysis requires Google Gemini and cloud access."
            );
        }
        if self.video_analysis != VideoAnalysis::None {
            anyhow::ensure!(
                self.video.kind != VideoKind::None,
                "Enable video recording for visual analysis."
            );
        }
        if self.video.kind == VideoKind::Region {
            let crop = self
                .video
                .crop
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Select a capture region."))?;
            anyhow::ensure!(
                crop.width >= 16
                    && crop.height >= 16
                    && crop.width <= 16_384
                    && crop.height <= 16_384,
                "Invalid region dimensions."
            );
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub microphone: Option<String>,
    pub desktop: Option<String>,
    pub mixed: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSegment {
    pub id: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub text: String,
    pub speaker: Option<String>,
    pub provisional: bool,
    #[serde(default)]
    pub edited: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EvidenceItem {
    pub text: String,
    #[serde(default)]
    pub sources: Vec<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    #[serde(default)]
    pub summary: Vec<EvidenceItem>,
    #[serde(default)]
    pub topics: Vec<EvidenceItem>,
    #[serde(default)]
    pub decisions: Vec<EvidenceItem>,
    #[serde(default)]
    pub tasks: Vec<EvidenceItem>,
    #[serde(default)]
    pub questions: Vec<EvidenceItem>,
    #[serde(default)]
    pub provisional: bool,
    #[serde(default)]
    pub generated_at: String,
}
impl AnalysisResult {
    pub fn validate_sources(&self, allowed: &HashSet<String>) -> anyhow::Result<()> {
        for item in self
            .summary
            .iter()
            .chain(&self.topics)
            .chain(&self.decisions)
            .chain(&self.tasks)
            .chain(&self.questions)
        {
            anyhow::ensure!(
                !item.sources.is_empty(),
                "AI result contains a claim without evidence; please retry."
            );
            anyhow::ensure!(
                item.sources.iter().all(|s| allowed.contains(s)),
                "AI result contains an unknown evidence reference; please retry."
            );
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Marker {
    pub at_ms: u64,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualEvidence {
    pub id: String,
    pub at_ms: u64,
    pub text: String,
    pub file: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Answer {
    pub question: String,
    pub answer: EvidenceItem,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meeting {
    pub id: String,
    pub created_at: String,
    pub updated_at: String,
    pub config: MeetingConfig,
    pub status: String,
    pub duration_ms: u64,
    pub tags: Vec<String>,
    pub notes: String,
    pub markers: Vec<Marker>,
    pub media: Vec<MediaSegment>,
    pub transcript: Vec<TranscriptSegment>,
    pub analysis: Option<AnalysisResult>,
    pub visuals: Vec<VisualEvidence>,
    pub answers: Vec<Answer>,
    pub warnings: Vec<String>,
    pub cloud_requests: u32,
    pub tokens: u64,
    pub video_files: Vec<String>,
    pub playback_file: Option<String>,
}
impl Meeting {
    pub fn new(config: MeetingConfig) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: now.clone(),
            updated_at: now,
            config,
            status: "ready".into(),
            duration_ms: 0,
            tags: vec![],
            notes: String::new(),
            markers: vec![],
            media: vec![],
            transcript: vec![],
            analysis: None,
            visuals: vec![],
            answers: vec![],
            warnings: vec![],
            cloud_requests: 0,
            tokens: 0,
            video_files: vec![],
            playback_file: None,
        }
    }
    pub fn source_ids(&self) -> HashSet<String> {
        self.transcript
            .iter()
            .map(|t| t.id.clone())
            .chain(self.visuals.iter().map(|v| v.id.clone()))
            .collect()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeetingSummary {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub duration_ms: u64,
    pub status: String,
    pub tags: Vec<String>,
    pub mode: Mode,
    pub offline: bool,
}
impl From<&Meeting> for MeetingSummary {
    fn from(m: &Meeting) -> Self {
        Self {
            id: m.id.clone(),
            title: m.config.title.clone(),
            created_at: m.created_at.clone(),
            duration_ms: m.duration_ms,
            status: m.status.clone(),
            tags: m.tags.clone(),
            mode: m.config.mode.clone(),
            offline: m.config.offline,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub locale: String,
    pub default_config: MeetingConfig,
    pub retention_days: Option<u32>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "de".into(),
            default_config: MeetingConfig::default(),
            retention_days: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub meeting_id: String,
    pub kind: String,
    pub payload: String,
    pub state: String,
    pub attempts: u32,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offline_profile_cannot_exfiltrate_audio() {
        let mut c = MeetingConfig {
            title: "Test".into(),
            consent: true,
            ..Default::default()
        };
        assert!(c.validate().is_ok());
        c.transcription.provider = Provider::Openai;
        assert!(c.validate().is_err());
    }
    #[test]
    fn evidence_must_point_to_existing_segments() {
        let mut a = AnalysisResult::default();
        a.tasks.push(EvidenceItem {
            text: "Ship".into(),
            sources: vec!["missing".into()],
            ..Default::default()
        });
        assert!(a.validate_sources(&HashSet::new()).is_err());
        assert!(a
            .validate_sources(&HashSet::from(["missing".into()]))
            .is_ok());
    }
}
