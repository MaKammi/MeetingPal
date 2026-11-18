export type Provider = "local" | "openai" | "anthropic" | "gemini";
export type Mode = "post" | "live" | "assistant";
export interface ModelSelection {
  provider: Provider;
  model: string;
}
export interface MeetingConfig {
  title: string;
  language: string;
  mode: Mode;
  offline: boolean;
  microphone: string | null;
  systemDevice: string | null;
  video: {
    kind: "none" | "screen" | "window" | "region" | "portal";
    id: string | null;
    crop: { x: number; y: number; width: number; height: number } | null;
  };
  transcription: ModelSelection;
  finalTranscription: ModelSelection | null;
  analysis: ModelSelection;
  vision: ModelSelection;
  videoAnalysis: "none" | "frames" | "native";
  maxRequests: number;
  consent: boolean;
  denoise: boolean;
}
export interface Settings {
  locale: string;
  defaultConfig: MeetingConfig;
  retentionDays: number | null;
  updateChecks: "onlineOnly" | "always" | "manual";
}
export interface EvidenceItem {
  text: string;
  sources: string[];
  owner?: string | null;
  dueDate?: string | null;
}
export interface AnalysisResult {
  summary: EvidenceItem[];
  topics: EvidenceItem[];
  decisions: EvidenceItem[];
  tasks: EvidenceItem[];
  questions: EvidenceItem[];
  provisional: boolean;
  generatedAt: string;
}
export interface TranscriptSegment {
  id: string;
  startMs: number;
  endMs: number;
  text: string;
  speaker: string | null;
  provisional: boolean;
  edited: boolean;
}
export interface MediaSegment {
  id: string;
  startMs: number;
  endMs: number;
  microphone: string | null;
  desktop: string | null;
  mixed: string;
}
export interface MeetingSummary {
  id: string;
  title: string;
  createdAt: string;
  durationMs: number;
  status: string;
  tags: string[];
  mode: Mode;
  offline: boolean;
}
export interface Meeting {
  id: string;
  createdAt: string;
  updatedAt: string;
  config: MeetingConfig;
  status: string;
  durationMs: number;
  tags: string[];
  notes: string;
  markers: { atMs: number; text: string }[];
  media: MediaSegment[];
  transcript: TranscriptSegment[];
  analysis: AnalysisResult | null;
  visuals: { id: string; atMs: number; text: string; file: string }[];
  answers: { question: string; answer: EvidenceItem; createdAt: string }[];
  warnings: string[];
  cloudRequests: number;
  tokens: number;
  videoFiles: string[];
  playbackFile: string | null;
}
export interface ModelSpec {
  id: string;
  name: string;
  task: string;
  license: string;
  source: string;
  files: { filename: string; bytes: number; sha256: string; url: string }[];
  installed: boolean;
}
export interface Job {
  id: string;
  meetingId: string;
  kind: string;
  payload: string;
  state: string;
  attempts: number;
  error: string | null;
}
export interface Device {
  id: string;
  name: string;
  kind: string;
  isDefault: boolean;
}
export interface Source {
  id: string;
  name: string;
  kind: string;
  width: number;
  height: number;
}
export interface Sources {
  devices: Device[] | null;
  sources: Source[] | null;
  deviceError: string | null;
  sourceError: string | null;
}
export interface Bootstrap {
  settings: Settings;
  meetings: MeetingSummary[];
  models: ModelSpec[];
  runtimes: Record<string, boolean>;
  keys: Record<string, boolean>;
  dataDirectory: string;
  activeMeeting: string | null;
}
export interface Progress {
  id: string;
  received: number;
  total: number;
  state: string;
  error: string | null;
}
export interface Levels {
  microphone: number;
  desktop: number;
  duration_ms: number;
}
export const defaultConfig = (): MeetingConfig => ({
  title: "",
  language: "auto",
  mode: "post",
  offline: true,
  microphone: null,
  systemDevice: null,
  video: { kind: "none", id: null, crop: null },
  transcription: { provider: "local", model: "whisper-small" },
  finalTranscription: null,
  analysis: { provider: "local", model: "qwen-text" },
  vision: { provider: "local", model: "qwen-vision" },
  videoAnalysis: "none",
  maxRequests: 2000,
  consent: false,
  denoise: true,
});
export function duration(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return `${seconds >= 3600 ? `${Math.floor(seconds / 3600)}:` : ""}${String(Math.floor(seconds / 60) % 60).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}
export function bytes(value: number) {
  return value >= 1e9
    ? `${(value / 1e9).toFixed(1)} GB`
    : `${Math.round(value / 1e6)} MB`;
}
export function disclosure(config: MeetingConfig) {
  const transmissions: { task: string; provider: Provider; data: string }[] =
    [];
  if (config.transcription.provider !== "local")
    transmissions.push({
      task: "transcription",
      provider: config.transcription.provider,
      data: "audio",
    });
  if (config.analysis.provider !== "local")
    transmissions.push({
      task: "analysis",
      provider: config.analysis.provider,
      data: "transcript",
    });
  if (
    config.finalTranscription &&
    config.finalTranscription.provider !== "local" &&
    config.finalTranscription.provider !== config.transcription.provider
  )
    transmissions.push({
      task: "final-transcription",
      provider: config.finalTranscription.provider,
      data: "audio",
    });
  if (config.videoAnalysis !== "none" && config.vision.provider !== "local")
    transmissions.push({
      task: "vision",
      provider: config.vision.provider,
      data: config.videoAnalysis === "native" ? "video + audio" : "images",
    });
  return transmissions;
}
