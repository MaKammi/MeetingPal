import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  AudioLines,
  Plus,
  Library,
  Cpu,
  Settings2,
  Search,
  Upload,
  ArrowUpRight,
  ArrowLeft,
  Clock3,
  ShieldCheck,
  Mic,
  Monitor,
  Video,
  FileText,
  Sparkles,
  Check,
  ChevronRight,
  Download,
  X,
  Pause,
  Play,
  Square,
  Bookmark,
  Send,
  RefreshCw,
  Trash2,
  FolderOpen,
  AlertCircle,
  Globe,
  HardDrive,
  Languages,
  CheckCircle2,
  LoaderCircle,
} from "lucide-react";
import { api, desktop } from "./api";
import { VideoSourceField } from "./VideoSourcePicker";
import { Modal } from "./Modal";
import { diagnostic } from "./diagnostics";
import capabilities from "../resources/model-capabilities.json";
import { useUpdates, updateError } from "./useUpdates";
import { cloudKeyChanged, useCloudModels } from "./useCloudModels";
import { version as appVersion } from "../package.json";
import { bytes, defaultConfig, disclosure, duration, editableConfig, withSelectedProviders } from "./types";
import type {
  Bootstrap,
  Meeting,
  MeetingConfig,
  MeetingSummary,
  Settings,
  ModelSelection,
  ModelSpec,
  Mode,
  Provider,
  Sources,
  Progress,
  Levels,
  Job,
  EvidenceItem,
  TranscriptSegment,
  Translate,
} from "./types";

const providerNames: Record<Provider, string> = {
  local: "Lokal",
  openai: "OpenAI",
  anthropic: "Anthropic",
  gemini: "Google Gemini",
};
function App() {
  const [boot, setBoot] = useState<Bootstrap | null>(null);
  const updates = useUpdates(boot?.settings, !!boot?.activeMeeting);
  const [page, setPage] = useState<
    "library" | "models" | "settings" | "meeting"
  >("library");
  const [meeting, setMeeting] = useState<Meeting | null>(null);
  const [search, setSearch] = useState("");
  const [matchedIds, setMatchedIds] = useState<Set<string> | null>(null);
  const [meetings, setMeetings] = useState<MeetingSummary[]>([]);
  const [setup, setSetup] = useState<{ file?: string } | null>(null);
  const [notice, setNotice] = useState("");
  const [busy, setBusy] = useState(false);
  const [levels, setLevels] = useState<Levels>({
    microphone: 0,
    desktop: 0,
    duration_ms: 0,
  });
  const [progress, setProgress] = useState<Record<string, Progress>>({});
  const currentId = useRef<string | null>(null);
  const librarySearch = useRef<HTMLInputElement>(null);
  useEffect(() => {
    const searchLibrary = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k" && !document.querySelector("dialog[open]")) {
        event.preventDefault();
        setPage("library");
        requestAnimationFrame(() => librarySearch.current?.focus());
      }
    };
    window.addEventListener("keydown", searchLibrary);
    return () => window.removeEventListener("keydown", searchLibrary);
  }, []);
  const t: Translate = (de, en) => (boot?.settings.locale === "en" ? en : de);
  useEffect(() => {
    document.documentElement.lang = boot?.settings.locale === "en" ? "en" : "de";
  }, [boot?.settings.locale]);
  const load = useCallback(async () => {
    const data = await api<Bootstrap>("bootstrap");
    setBoot(data);
    setMeetings(data.meetings);
  }, []);
  useEffect(() => {
    if (!search) {
      setMatchedIds(null);
      return;
    }
    let disposed = false;
    const timer = setTimeout(() => {
      api<MeetingSummary[]>("list_meetings", { query: search })
        .then((rows) => {
          if (!disposed) setMatchedIds(new Set(rows.map((m) => m.id)));
        })
        .catch((e) => setNotice(String(e)));
    }, 200);
    return () => {
      disposed = true;
      clearTimeout(timer);
    };
  }, [search, meetings]);
  const refreshMeeting = useCallback(async (id: string) => {
    const data = await api<Meeting>("get_meeting", { id });
    if (currentId.current === id) setMeeting(data);
  }, []);
  useEffect(() => {
    load().catch((e) => setNotice(String(e)));
  }, [load]);
  useEffect(() => {
    currentId.current = meeting?.id ?? null;
  }, [meeting?.id]);
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const cleanups: (() => void)[] = [];
    async function connect() {
      const listeners = [
        listen<string>("meeting-changed", (e) => {
          api<MeetingSummary[]>("list_meetings", { query: "" })
            .then(setMeetings)
            .catch(() => {});
          if (currentId.current === e.payload)
            refreshMeeting(e.payload).catch(() => {});
        }),
        listen<{ meetingId: string; event: Levels & { type: string } }>(
          "capture-event",
          (e) => {
            if (e.payload.event.type === "level") setLevels(e.payload.event);
          },
        ),
        listen<string>("recording-ended", () => {
          setBoot((b) => (b ? { ...b, activeMeeting: null } : b));
        }),
        listen<Progress>("model-download", (e) => {
          setProgress((p) => ({ ...p, [e.payload.id]: e.payload }));
          if (e.payload.state === "complete") load().catch(() => {});
        }),
        listen<string>("app-notice", (e) => setNotice(e.payload)),
      ];
      for (const pending of listeners) {
        const cleanup = await pending;
        if (disposed) cleanup();
        else cleanups.push(cleanup);
      }
    }
    connect().catch((e) => setNotice(String(e)));
    return () => {
      disposed = true;
      cleanups.forEach((f) => f());
    };
  }, [load, refreshMeeting]);
  const act = async (work: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await work();
    } catch (e) {
      setNotice(String(e));
    } finally {
      setBusy(false);
    }
  };
  const selectMeeting = async (id: string) => {
    currentId.current = id;
    await refreshMeeting(id);
    setPage("meeting");
  };
  const importFile = () =>
    act(async () => {
      if (!desktop)
        throw new Error(
          t("Bitte öffne die Desktop-App.", "Please open the desktop app."),
        );
      const file = await open({
        multiple: false,
        filters: [
          {
            name: "Audio / Video",
            extensions: [
              "wav",
              "mp3",
              "m4a",
              "flac",
              "ogg",
              "opus",
              "webm",
              "mp4",
              "mkv",
              "mov",
              "aac",
            ],
          },
        ],
      });
      if (file) setSetup({ file });
    });
  const begin = (config: MeetingConfig, test: boolean) =>
    act(async () => {
      const data = setup?.file
        ? await api<Meeting>("import_media", { path: setup.file, config })
        : await api<Meeting>("start_recording", { config, test });
      currentId.current = data.id;
      setMeeting(data);
      setPage("meeting");
      setSetup(null);
      setLevels({ microphone: 0, desktop: 0, duration_ms: 0 });
      if (!setup?.file)
        setBoot((b) => (b ? { ...b, activeMeeting: data.id } : b));
      else await load();
    });
  if (!boot)
    return (
      <div className="loading">
        <AudioLines size={40} />
        <p>MeetingPal</p>
        {notice ? (
          <p role="alert">{diagnostic(notice, t)}</p>
        ) : (
          <LoaderCircle className="spin" />
        )}
      </div>
    );
  const filtered = !search
    ? meetings
    : meetings.filter((m) => matchedIds?.has(m.id));
  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a
          className="brand"
          href="#"
          onClick={(e) => {
            e.preventDefault();
            setPage("library");
          }}
        >
          <span className="brand-mark">
            <AudioLines size={25} />
          </span>
          Meeting<span>Pal</span>
        </a>
        <button
          className="primary new-meeting"
          onClick={() => setSetup({})}
          disabled={!!boot.activeMeeting}
        >
          <Plus size={18} />
          {t("Meeting aufnehmen", "Record meeting")}
        </button>
        <div className="nav-label">{t("ARBEITSBEREICH", "WORKSPACE")}</div>
        <nav aria-label={t("Hauptnavigation", "Main navigation")}>
          <button
            className={
              page === "library" || page === "meeting" ? "selected" : ""
            }
            aria-current={page === "library" || page === "meeting" ? "page" : undefined}
            onClick={() => setPage("library")}
          >
            <Library size={19} />
            {t("Meine Meetings", "My meetings")}
            <span className="nav-count">{meetings.length}</span>
          </button>
          <button
            aria-current={page === "models" ? "page" : undefined}
            className={page === "models" ? "selected" : ""}
            onClick={() => setPage("models")}
          >
            <Cpu size={19} />
            {t("KI-Modelle", "AI models")}
            <span className="tiny-dot" />
          </button>
          <button
            aria-current={page === "settings" ? "page" : undefined}
            className={page === "settings" ? "selected" : ""}
            onClick={() => setPage("settings")}
          >
            <Settings2 size={19} />
            {t("Einstellungen", "Settings")}
          </button>
        </nav>
        {boot.activeMeeting && (
          <button
            className="active-session"
            onClick={() => act(() => selectMeeting(boot.activeMeeting!))}
          >
            <span className="record-dot" />
            <span>
              {t("Aufnahme geöffnet", "Recording open")}
              <small>{duration(levels.duration_ms)}</small>
            </span>
            <ChevronRight size={16} />
          </button>
        )}
        <div className="sidebar-bottom">
          <div className="privacy-card">
            <ShieldCheck size={22} />
            <strong>
              {t("Deine Meetings. Deine Daten.", "Your meetings. Your data.")}
            </strong>
            <p>
              {t(
                "Lokal gespeichert. Du entscheidest, welche KI mitarbeitet.",
                "Stored locally. You choose which AI takes part.",
              )}
            </p>
          </div>
          <div className="version">
            <span className="online-dot" />
            MeetingPal <span>v{appVersion}</span>
          </div>
        </div>
      </aside>
      <main>
        <header className="topbar">
          <span>
            {page === "library"
              ? t("Meine Meetings", "My meetings")
              : page === "models"
                ? t("KI-Modelle", "AI models")
                : page === "settings"
                  ? t("Einstellungen", "Settings")
                  : t("Meeting-Workspace", "Meeting workspace")}
          </span>
          <div className="topbar-right">
            <span className="badge neutral">
              <HardDrive size={13} />
              {t("Lokal gespeichert", "Stored locally")}
            </span>
            <button
              className="avatar"
              aria-label={t("Einstellungen öffnen", "Open settings")}
              onClick={() => setPage("settings")}
            >
              MP
            </button>
          </div>
        </header>
        {updates.version && page !== "settings" && (
          <div className="preview-banner update-banner">
            <Download size={16} />
            <span role="status">{updates.status === "installing"
              ? t("Update wird installiert …", "Installing update…") + (updates.percent === null ? "" : ` ${updates.percent}%`)
              : t(`MeetingPal ${updates.version} ist verfügbar.`, `MeetingPal ${updates.version} is available.`)}</span>
            <button className="secondary" disabled={!!boot.activeMeeting || updates.status === "checking" || updates.status === "installing"} onClick={() => void updates.install()}>
              {t(`Version ${updates.version} installieren und neu starten`, `Install ${updates.version} and restart`)}
            </button>
            {boot.activeMeeting && <span>{t("Updates sind nach der Aufnahme verfügbar.", "Updates are available after recording.")}</span>}
            {updates.error && <span role="alert">{updateError(updates.error, t)}</span>}
          </div>
        )}
        {!desktop && (
          <div className="preview-banner">
            <Monitor size={15} />
            {t(
              "Web-Vorschau · Aufnahme und Dateiverarbeitung sind in der Desktop-App verfügbar.",
              "Web preview · Recording and file processing are available in the desktop app.",
            )}
          </div>
        )}
        <div className="page-content">
          {page === "library" && (
            <>
              <div className="page-heading">
                <div>
                  <div className="eyebrow">MeetingPal</div>
                  <h1>
                    {t(
                      "Mehr im Gespräch. Weniger im Protokoll.",
                      "More conversation. Less note-taking.",
                    )}
                  </h1>
                  <p>
                    {t(
                      "Halte fest, was zählt. MeetingPal kümmert sich um den Rest.",
                      "Capture what matters. MeetingPal takes care of the notes.",
                    )}
                  </p>
                </div>
                <button
                  className="secondary"
                  disabled={busy}
                  onClick={importFile}
                >
                  <Upload size={16} />
                  {t("Datei importieren", "Import file")}
                </button>
              </div>
              <section className="hero-card">
                <div className="hero-copy">
                  <span className="badge lavender">
                    <Sparkles size={13} />
                    {t(
                      "Dein persönlicher Meeting-Assistent",
                      "Your personal meeting assistant",
                    )}
                  </span>
                  <h2>
                    {t(
                      "Gute Gespräche verdienen",
                      "Good conversations deserve",
                    )}
                    <br />
                    {t("klare Ergebnisse.", "clear outcomes.")}
                  </h2>
                  <p>
                    {t(
                      "Aufnehmen, verstehen und weiterdenken. Mit deiner KI und voller Kontrolle über deine Daten.",
                      "Record, understand and move forward. With your AI and full control of your data.",
                    )}
                  </p>
                  <button
                    className="primary"
                    disabled={!!boot.activeMeeting}
                    onClick={() => setSetup({})}
                  >
                    <Mic size={17} />
                    {t("Neues Meeting starten", "Start a new meeting")}
                    <ArrowUpRight size={17} />
                  </button>
                  <div className="hero-features">
                    <span>
                      <Check size={13} />
                      {t("Mikrofon + Desktop", "Microphone + desktop")}
                    </span>
                    <span>
                      <Check size={13} />
                      {t("Cloud oder lokal", "Cloud or local")}
                    </span>
                    <span>
                      <Check size={13} />
                      {t("Video optional", "Optional video")}
                    </span>
                  </div>
                </div>
                <div className="hero-visual" aria-hidden="true">
                  <div className="visual-orbit orbit-one" />
                  <div className="visual-orbit orbit-two" />
                  <div className="floating-note">
                    <span className="note-icon">
                      <Check size={16} />
                    </span>
                    <div>
                      <strong>
                        {t("Entscheidungen festhalten", "Capture decisions")}
                      </strong>
                      <div className="fake-line" />
                      <div className="fake-line short" />
                    </div>
                  </div>
                  <div className="wave-tile">
                    <AudioLines size={76} strokeWidth={1.2} />
                  </div>
                  <div className="floating-tag">
                    <Sparkles size={16} />
                    {t(
                      "Aus Gesprächen wird Klarheit",
                      "Turn conversation into clarity",
                    )}
                  </div>
                </div>
              </section>
              <div className="stats-row">
                <Stat
                  icon={<Library size={19} />}
                  label={t(
                    "Meetings in deiner Bibliothek",
                    "Meetings in your library",
                  )}
                  value={String(meetings.length)}
                />
                <Stat
                  icon={<Clock3 size={19} />}
                  label={t("Aufgezeichnete Zeit", "Recorded time")}
                  value={duration(
                    meetings.reduce((n, m) => n + m.durationMs, 0),
                  )}
                />
                <Stat
                  icon={<Cpu size={19} />}
                  label={t("Lokale Modelle bereit", "Local models ready")}
                  value={String(boot.models.filter((m) => m.installed).length)}
                />
              </div>
              <div className="section-heading">
                <h2>
                  {t("Deine Meetings", "Your meetings")}
                  <span className="count-pill">{meetings.length}</span>
                </h2>
                <label className="search-box">
                  <Search size={16} />
                  <input
                    ref={librarySearch}
                    aria-keyshortcuts="Control+k Meta+k"
                    title={t("Bibliothek durchsuchen (Strg/⌘ + K)", "Search library (Ctrl/⌘ + K)")}
                    aria-label={t("Meetings durchsuchen", "Search meetings")}
                    value={search}
                    onChange={(e) => setSearch(e.target.value)}
                    placeholder={t(
                      "Titel, Tags oder Transkript …",
                      "Search titles, tags or transcript …",
                    )}
                  />
                </label>
              </div>
              {filtered.length ? (
                <div className="meeting-list">
                  {filtered.map((m) => (
                    <button
                      key={m.id}
                      className="meeting-row"
                      onClick={() => act(() => selectMeeting(m.id))}
                    >
                      <div className="meeting-icon">
                        <AudioLines size={22} />
                      </div>
                      <div className="meeting-row-title">
                        <strong>{m.title}</strong>
                        <span>
                          {new Date(m.createdAt).toLocaleDateString(
                            boot.settings.locale,
                            { day: "numeric", month: "long", year: "numeric" },
                          )}
                          {m.tags.length ? ` · ${m.tags.join(", ")}` : ""}
                        </span>
                      </div>
                      <span className="muted time">
                        <Clock3 size={14} />
                        {duration(m.durationMs)}
                      </span>
                      <span
                        className={`badge ${m.status === "complete" ? "green" : m.status === "needs_attention" ? "amber" : "neutral"}`}
                      >
                        {statusLabel(m.status, t)}
                      </span>
                      <ChevronRight size={18} />
                    </button>
                  ))}
                </div>
              ) : (
                <div className="empty-state">
                  <span className="empty-icon">
                    <FileText size={28} />
                  </span>
                  <h3>
                    {search
                      ? t("Keine passenden Meetings", "No matching meetings")
                      : t(
                          "Hier beginnt dein nächstes gutes Meeting.",
                          "Your next great meeting starts here.",
                        )}
                  </h3>
                  <p>
                    {t(
                      "Starte eine Aufnahme oder importiere eine bestehende Datei.",
                      "Start a recording or import an existing file.",
                    )}
                  </p>
                  <button className="text-button" onClick={() => setSetup({})}>
                    {t("Erstes Meeting aufnehmen", "Record your first meeting")}
                    <ChevronRight size={15} />
                  </button>
                </div>
              )}
            </>
          )}
          {page === "models" && (
            <ModelsPage
              models={boot.models}
              progress={progress}
              t={t}
              act={act}
              refresh={load}
              notify={setNotice}
            />
          )}
          {page === "settings" && (
            <SettingsPage
              updates={updates}
              boot={boot}
              t={t}
              act={act}
              onSaved={load}
              notify={setNotice}
            />
          )}
          {page === "meeting" && meeting && (
            <MeetingPage
              key={meeting.id}
              meeting={meeting}
              levels={levels}
              t={t}
              busy={busy}
              act={act}
              update={setMeeting}
              onBack={() => setPage("library")}
              onDeleted={() => {
                currentId.current = null;
                setMeeting(null);
                setPage("library");
                load().catch(() => {});
              }}
            />
          )}
        </div>
      </main>
      {setup && (
        <Setup
          config={boot.settings.defaultConfig}
          models={boot.models}
          file={setup.file}
          t={t}
          busy={busy}
          onClose={() => !busy && setSetup(null)}
          onStart={begin}
        />
      )}
      {notice && (
        <div className="toast" role="alert">
          <AlertCircle size={20} />
          <span>{diagnostic(notice, t)}</span>
          <button
            className="icon-button"
            aria-label={t("Meldung schließen", "Dismiss notification")}
            onClick={() => setNotice("")}
          >
            <X size={17} />
          </button>
        </div>
      )}
    </div>
  );
}

function Stat({
  icon,
  label,
  value,
}: {
  icon: React.ReactNode;
  label: string;
  value: string;
}) {
  return (
    <div className="stat">
      <span className="stat-icon">{icon}</span>
      <div>
        <strong>{value}</strong>
        <span>{label}</span>
      </div>
    </div>
  );
}
function statusLabel(status: string, t: Translate) {
  const labels: Record<string, [string, string]> = {
    ready: ["Bereit", "Ready"],
    transcribe: ["Transkription", "Transcription"],
    finalize: ["Abschließende Auswertung", "Final analysis"],
    live_analysis: ["Live-Auswertung", "Live analysis"],
    pending: ["Wartet", "Queued"],
    running: ["In Arbeit", "Running"],
    done: ["Abgeschlossen", "Completed"],
    starting: ["Startet …", "Starting …"],
    recording: ["Aufnahme läuft", "Recording"],
    paused: ["Pausiert", "Paused"],
    processing: ["Wird ausgewertet", "Processing"],
    complete: ["Aufbereitet", "Ready"],
    recorded: ["Aufgezeichnet", "Recorded"],
    recovered: ["Wiederhergestellt", "Recovered"],
    needs_attention: ["Aktion erforderlich", "Needs attention"],
    importing: ["Wird importiert", "Importing"],
    failed: ["Fehlgeschlagen", "Failed"],
  };
  const pair = labels[status];
  return pair ? t(...pair) : status;
}

function ModelPicker({
  label,
  value,
  onChange,
  task,
  t,
  online,
  nativeVideo = false,
}: {
  online: boolean;
  nativeVideo?: boolean;
  label: string;
  value: ModelSelection;
  onChange: (v: ModelSelection) => void;
  task: string;
  t: Translate;
}) {
  const {models, error, loading, refresh} = useCloudModels(value.provider, online);
  const [manualProvider, setManualProvider] = useState<Provider | null>(null);
  const [probing, setProbing] = useState(false);
  const [receipt, setReceipt] = useState<{ key: string; text: string } | null>(
    null,
  );
  const input = task.endsWith("transcription")
    ? "audio"
    : task === "vision"
      ? nativeVideo
        ? "video"
        : "image"
      : "text";
  const key = [value.provider, value.model, input].join("/");
  const features =
    capabilities.find(
      (m) => m.provider === value.provider && m.model === value.model,
    )?.features ?? models.find(m => m.id === value.model)?.inputs ?? [];
  const featureNames: Record<string, string> = {
    audio: t("Transkription", "Transcription"),
    text: t("Text", "Text"),
    image: t("Bilder", "Images"),
    video: t("Videodateien", "Video files"),
    timestamps: t("Zeitmarken", "Timestamps"),
    speakerLabels: t("Sprecher pro Abschnitt", "Speakers per section"),
    audioStream: t("Live-Vorschau", "Streaming preview"),
  };
  const suggestions = models.filter(model => model.inputs.includes(input));
  const first = suggestions[0]?.id;
  const manual = manualProvider === value.provider;
  useEffect(() => {
    if (online && !loading && !error && !manual && !value.model && first) onChange({provider: value.provider, model: first});
  }, [online, loading, error, manual, value.provider, value.model, first, onChange]);
  const providers: Provider[] = nativeVideo
    ? ["gemini"]
    : task.endsWith("transcription")
      ? ["local", "openai", "gemini"]
      : ["local", "openai", "anthropic", "gemini"];
  const local = task.endsWith("transcription")
    ? ["whisper-small", "whisper-turbo"]
    : task === "vision"
      ? ["qwen-vision"]
      : ["qwen-text", "qwen-vision"];
  return (
    <div className="model-picker">
      <label>{label}</label>
      <div className="field-row">
        <select
          aria-label={`${label} ${t("Anbieter", "provider")}`}
          value={value.provider}
          onChange={(e) => {
            const provider = e.target.value as Provider;
            setManualProvider(null);
            setReceipt(null);
            onChange({ provider, model: provider === "local" ? local[0] : "" });
          }}
        >
          {providers.map((p) => (
            <option key={p} value={p}>
              {p === "local"
                ? t("Lokal auf diesem Gerät", "Local on this device")
                : providerNames[p]}
            </option>
          ))}
        </select>
        {value.provider === "local" ? (
          <select
            aria-label={`${label} ${t("Modell", "model")}`}
            value={value.model}
            onChange={(e) => onChange({ ...value, model: e.target.value })}
          >
            {local.map((m) => (
              <option key={m}>{m}</option>
            ))}
          </select>
        ) : (
          <>
            {suggestions.length > 0 && !manual ? (
              <select
                aria-label={`${label} ${t("Modell", "model")}`}
                value={value.model || first}
                onChange={(e) => {
                  if (e.target.value === "__custom") {
                    setManualProvider(value.provider);
                    onChange({ ...value, model: "" });
                  } else onChange({ ...value, model: e.target.value });
                }}
              >
                {suggestions.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.id}
                  </option>
                ))}
                {value.model &&
                  !suggestions.some((m) => m.id === value.model) && (
                    <option value={value.model}>
                      {value.model} ·{" "}
                      {t("Gespeicherte Auswahl", "Saved selection")}
                    </option>
                  )}
                <option value="__custom">
                  {t("Eigene Modell-ID eingeben …", "Enter a custom model ID…")}
                </option>
              </select>
            ) : (
              <input
                aria-label={`${label} ${t("Modell", "model")}`}
                value={value.model}
                placeholder={
                  loading
                    ? t("Modelle werden geladen …", "Loading models…")
                    : t("Modell-ID eingeben", "Enter model ID")
                }
                onChange={(e) => {
                  setManualProvider(value.provider);
                  onChange({ ...value, model: e.target.value });
                }}
              />
            )}
            {manual && suggestions.length > 0 && (
              <button
                type="button"
                className="text-button"
                onClick={() => setManualProvider(null)}
              >
                {t("Zur Liste", "Show list")}
              </button>
            )}
            <button
              type="button"
              className="icon-button"
              title={t("Modelle vom Anbieter laden", "Load provider models")}
              disabled={loading || !online}
              onClick={refresh}
            >
              {loading ? (
                <LoaderCircle size={16} className="spin" />
              ) : (
                <RefreshCw size={16} />
              )}
            </button>
          </>
        )}
      </div>
      {!!features.length && (
        <small>
          {features.map((feature) => featureNames[feature]).join(" · ")}
        </small>
      )}
      {online &&
        value.provider !== "local" &&
        value.model.trim() &&
        desktop && (
          <button
            type="button"
            className="text-button"
            disabled={probing}
            title={t(
              "Verwendet feste Testdaten und kann API-Kosten verursachen.",
              "Uses fixed sample data and may incur API charges.",
            )}
            onClick={async () => {
              setProbing(true);
              try {
                const result = await api<{
                  elapsedMs: number;
                  requests: number;
                }>("probe_model", { selection: value, task: input });
                setReceipt({
                  key,
                  text:
                    t("Test bestanden", "Test passed") +
                    " · " +
                    (result.elapsedMs / 1000).toFixed(1) +
                    " s",
                });
              } catch (e) {
                setReceipt({ key, text: String(e) });
              } finally {
                setProbing(false);
              }
            }}
          >
            {probing
              ? t("Test läuft …", "Testing …")
              : t(
                  "Mit Testdaten beim Anbieter prüfen",
                  "Test with sample data at provider",
                )}
          </button>
        )}
      {online && desktop && value.provider !== "local" && !loading && !error && !suggestions.length && (
        <small role="status">{t("Der Anbieter meldet keine passenden Modelle. Du kannst die Liste aktualisieren oder eine Modell-ID eingeben.", "The provider returned no compatible models. Refresh the list or enter a model ID.")}</small>
      )}
      {receipt?.key === key && (
        <small role="status">{diagnostic(receipt.text, t)}</small>
      )}
      {error && (
        <small className="error-text" role="alert">
          {diagnostic(error, t)}
        </small>
      )}
    </div>
  );
}

function Setup({
  config,
  models,
  file,
  t,
  busy,
  onClose,
  onStart,
}: {
  config: MeetingConfig;
  models: ModelSpec[];
  file?: string;
  t: Translate;
  busy: boolean;
  onClose: () => void;
  onStart: (c: MeetingConfig, test: boolean) => void;
}) {
  const [c, setC] = useState<MeetingConfig>(() => ({
    ...editableConfig(config),
    consent: false,
    title: file
      ? file
          .split(/[/\\]/)
          .pop()!
          .replace(/\.[^.]+$/, "")
      : "",
    ...(file
      ? {
          mode: "post" as Mode,
          video: { kind: "portal" as const, id: null, crop: null },
        }
      : {}),
  }));
  const [confirmGemini, setConfirmGemini] = useState(false);
  function selectGeminiVideo() {
    setC((current) => ({
      ...current, offline: false, videoAnalysis: "native",
      vision: current.vision.provider === "gemini" ? current.vision : {provider: "gemini", model: ""},
    }));
    setConfirmGemini(false);
  }
  const [sources, setSources] = useState<Sources>({
    devices: [],
    sources: [],
    deviceError: null,
    sourceError: null,
  });
  const [loading, setLoading] = useState(true);
  const patch = (changes: Partial<MeetingConfig>) =>
    setC((current) => withSelectedProviders({ ...current, ...changes }));
  useEffect(() => {
    if (file) {
      setLoading(false);
      return;
    }
    api<Sources>("capture_sources")
      .then(setSources)
      .catch((e) => setSources((s) => ({ ...s, deviceError: String(e) })))
      .finally(() => setLoading(false));
  }, [file]);
  const transmissions = disclosure(c);
  const missing = [
    c.transcription,
    ...(c.finalTranscription ? [c.finalTranscription] : []),
    c.analysis,
    ...(c.videoAnalysis !== "none" ? [c.vision] : []),
  ].filter(
    (s) =>
      s.provider === "local" &&
      !models.find((m) => m.id === s.model)?.installed,
  );
  const valid =
    c.title.trim() &&
    c.consent &&
    c.transcription.model &&
    c.analysis.model &&
    (!c.finalTranscription || c.finalTranscription.model.trim()) &&
    (c.videoAnalysis === "none" || c.vision.model.trim()) &&
    (c.videoAnalysis !== "native" || c.vision.provider === "gemini") &&
    (c.video.kind === "none" || file || c.video.id);
  return (
    <Modal className="setup-modal" labelledBy="setup-title" onClose={() => { if (!busy) onClose(); }}>
        <div className="modal-header">
          <div>
            <span className="eyebrow">
              {t("ALLES BEREIT FÜR DEIN MEETING", "READY FOR YOUR MEETING")}
            </span>
            <h2 id="setup-title">
              {file
                ? t("Aufnahme importieren", "Import recording")
                : t("Ein guter Start.", "A good start.")}
            </h2>
          </div>
          <button
            className="icon-button"
            aria-label={t("Schließen", "Close")}
            onClick={onClose}
            disabled={busy}
          >
            <X />
          </button>
        </div>
        <div className="setup-body">
          <div className="setup-main">
            <label className="field-label">
              {t("Meeting-Titel", "Meeting title")}
              <input
                autoFocus
                value={c.title}
                onChange={(e) => patch({ title: e.target.value })}
                placeholder={t(
                  "z. B. Produktplanung · November",
                  "e.g. Product planning · November",
                )}
                maxLength={250}
              />
            </label>
            {file ? (
              <div className="info-box">
                <FileText size={18} />
                <span>{file}</span>
              </div>
            ) : (
              <>
                <h3>
                  {t(
                    "Wie soll MeetingPal dich unterstützen?",
                    "How can MeetingPal help?",
                  )}
                </h3>
                <div className="mode-grid">
                  {(["post", "live", "assistant"] as Mode[]).map((mode, i) => (
                    <button
                      className={`mode-option ${c.mode === mode ? "chosen" : ""}`}
                      key={mode}
                      onClick={() => patch({ mode })}
                    >
                      {i === 0 ? (
                        <FileText size={20} />
                      ) : i === 1 ? (
                        <AudioLines size={20} />
                      ) : (
                        <Sparkles size={20} />
                      )}
                      <strong>
                        {
                          [
                            t("Danach auswerten", "Analyze afterwards"),
                            t("Live-Transkript", "Live transcript"),
                            t("Live-Assistenz", "Live assistance"),
                          ][i]
                        }
                      </strong>
                      <span>
                        {
                          [
                            t("In Ruhe aufnehmen", "Focus on recording"),
                            t("Gesagtes mitlesen", "Follow what is said"),
                            t("Mitdenken lassen", "Get help as you go"),
                          ][i]
                        }
                      </span>
                      {c.mode === mode && (
                        <CheckCircle2 size={15} className="mode-check" />
                      )}
                    </button>
                  ))}
                </div>
                <h3>
                  {t(
                    "Was möchtest du aufnehmen?",
                    "What would you like to record?",
                  )}
                </h3>
                <div className="two-columns">
                  <label className="field-label">
                    <span>
                      <Mic size={15} />
                      {t("Mikrofon", "Microphone")}
                    </span>
                    <select
                      value={c.microphone ?? ""}
                      onChange={(e) =>
                        patch({ microphone: e.target.value || null })
                      }
                    >
                      <option value="">
                        {t("Standard-Mikrofon", "Default microphone")}
                      </option>
                      {sources.devices
                        ?.filter((d) => d.kind === "microphone")
                        .map((d) => (
                          <option key={d.id} value={d.id}>
                            {d.id === "sck-desktop" ? t("Systemton (ScreenCaptureKit)", "System audio (ScreenCaptureKit)") : d.name}
                          </option>
                        ))}
                    </select>
                  </label>
                  <label className="field-label">
                    <span>
                      <Monitor size={15} />
                      {t("Desktop-Ton", "Desktop audio")}
                    </span>
                    <select
                      value={c.systemDevice ?? ""}
                      onChange={(e) =>
                        patch({ systemDevice: e.target.value || null })
                      }
                    >
                      <option value="">
                        {t("Standard-Ausgabegerät", "Default output device")}
                      </option>
                      {sources.devices
                        ?.filter((d) => d.kind === "desktop")
                        .map((d) => (
                          <option key={d.id} value={d.id}>
                            {d.id === "sck-desktop" ? t("Systemton (ScreenCaptureKit)", "System audio (ScreenCaptureKit)") : d.name}
                          </option>
                        ))}
                    </select>
                  </label>
                </div>
                {loading && (
                  <small className="muted">
                    {t("Geräte werden geladen …", "Loading devices …")}
                  </small>
                )}
                {sources.deviceError && (
                  <small className="error-text">{diagnostic(sources.deviceError, t)}</small>
                )}
                <label className="checkbox-row">
                  <input
                    type="checkbox"
                    checked={c.video.kind !== "none"}
                    onChange={(e) =>
                      patch({
                        video: {
                          kind: e.target.checked ? "screen" : "none",
                          id: null,
                          crop: null,
                        },
                        videoAnalysis: "none",
                      })
                    }
                  />
                  <Video size={17} />
                  <span>
                    {t(
                      "Bildschirm oder Fenster zusätzlich aufnehmen",
                      "Also record a screen or window",
                    )}
                    <small>
                      {t(
                        "Der Desktop-Ton kann auch andere Anwendungen enthalten.",
                        "Desktop audio may include other applications.",
                      )}
                    </small>
                  </span>
                </label>
                {c.video.kind !== "none" && (
                  <VideoSourceField
                    value={c.video}
                    sources={sources.sources ?? []}
                    onChange={(video) => patch({ video })}
                    t={t}
                  />
                )}
              </>
            )}
            <h3>{t("Deine KI, deine Wahl.", "Your AI, your choice.")}</h3>
            <ModelPicker
              label={t("Transkription", "Transcription")}
              task="transcription"
              online={!c.offline}
              value={c.transcription}
              t={t}
              onChange={(transcription) => patch({ transcription })}
            />
            {c.mode !== "post" && !c.offline && capabilities.some((m) => m.provider === c.transcription.provider && m.model === c.transcription.model && m.features.includes("audioStream")) && (
              <label className="checkbox-row"><input type="checkbox" checked={!!c.streamTranscription} onChange={(e) => patch({streamTranscription:e.target.checked})}/>
                {t("Wörter während der Transkription anzeigen", "Show words as each section is transcribed")}
              </label>
            )}
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={!!c.finalTranscription}
                onChange={(e) =>
                  patch({
                    finalTranscription: e.target.checked
                      ? { provider: "local", model: "whisper-turbo" }
                      : null,
                  })
                }
              />
              {t(
                "Anderes Modell für das endgültige Transkript",
                "Use a different model for the final transcript",
              )}
            </label>
            {c.finalTranscription && (
              <ModelPicker
                label={t("Endgültiges Transkript", "Final transcript")}
                task="final-transcription"
                online={!c.offline}
                value={c.finalTranscription}
                t={t}
                onChange={(finalTranscription) => patch({ finalTranscription })}
              />
            )}
            <p className="small muted">
              {t(
                "Nach dem Meeting wird Audio mit größerem Kontext verarbeitet. Ein Live-Profil benötigt dafür zusätzliche Rechenzeit bzw. API-Anfragen.",
                "After the meeting, audio is processed with larger context. Live profiles require additional processing time or API calls for this pass.",
              )}
            </p>
            <ModelPicker
              label={t("Auswertung & Assistenz", "Analysis & assistance")}
              task="analysis"
              online={!c.offline}
              value={c.analysis}
              t={t}
              onChange={(analysis) => patch({ analysis })}
            />
            {c.video.kind !== "none" && (
              <>
                <label className="field-label">
                  {t("Visuelle Auswertung", "Visual analysis")}
                  <select
                    value={c.videoAnalysis}
                    onChange={(e) => {
                      const mode = e.target.value as MeetingConfig["videoAnalysis"];
                      if (mode === "native") {
                        if (c.offline) setConfirmGemini(true);
                        else selectGeminiVideo();
                      } else patch({videoAnalysis: mode});
                    }}
                  >
                    <option value="none">
                      {t("Nur den Ton auswerten", "Analyze audio only")}
                    </option>
                    <option value="frames">
                      {t(
                        "Ausgewählte Bilder ergänzen",
                        "Include selected frames",
                      )}
                    </option>
                    <option value="native">
                      {t(
                        "Videoclips mit Ton · Gemini",
                        "Video clips with audio · Gemini",
                      )}
                    </option>
                  </select>
                </label>
                {c.videoAnalysis !== "none" && (
                  <ModelPicker
                    label={t("Bild-/Videomodell", "Vision model")}
                    task="vision"
                    online={!c.offline}
                    nativeVideo={c.videoAnalysis === "native"}
                    value={c.vision}
                    t={t}
                    onChange={(vision) => patch({ vision })}
                  />
                )}
              </>
            )}
            <div className="two-columns">
              <label className="field-label">
                {t("Gesprächssprache", "Meeting language")}
                <select
                  value={c.language}
                  onChange={(e) => patch({ language: e.target.value })}
                >
                  <option value="auto">
                    {t("Automatisch erkennen", "Detect automatically")}
                  </option>
                  <option value="de">Deutsch</option>
                  <option value="en">English</option>
                  <option value="fr">Français</option>
                  <option value="es">Español</option>
                  <option value="it">Italiano</option>
                  <option value="nl">Nederlands</option>
                </select>
              </label>
              <label className="field-label">
                {t("Maximale Cloud-Anfragen", "Maximum cloud requests")}
                <input
                  type="number"
                  min={1}
                  max={100000}
                  value={c.maxRequests}
                  onChange={(e) =>
                    patch({ maxRequests: Number(e.target.value) })
                  }
                />
              </label>
            </div>
            <label className="checkbox-row">
              <input
                type="checkbox"
                checked={c.denoise}
                onChange={(e) => patch({ denoise: e.target.checked })}
              />
              {t(
                "Echo im abgeleiteten Audiomix reduzieren",
                "Reduce echo in the derived audio mix",
              )}
            </label>
          </div>
          <aside className="setup-summary">
            <div className="summary-icon">
              <ShieldCheck size={28} />
            </div>
            <h3>{t("Du behältst die Kontrolle.", "You stay in control.")}</h3>
            <p>
              {t(
                "Die Aufnahme bleibt auf deinem Gerät. Die gewählten KI-Aufgaben bestimmen, was übertragen wird.",
                "Your recording stays on your device. Your chosen AI tasks determine what is sent.",
              )}
            </p>
            <div className="disclosure-list">
              {transmissions.length ? (
                transmissions.map((d, i) => (
                  <div key={i}>
                    <Globe size={15} />
                    <span>
                      <strong>{providerNames[d.provider]}</strong>
                      <small>{{audio:t("Audiomix", "Mixed audio"), transcript:t("Transkript", "Transcript"), images:t("Bilder", "Images"), "video + audio":t("Videoclip mit Ton · Datei-Upload", "Video with audio · file upload")}[d.data]}</small>
                    </span>
                  </div>
                ))
              ) : (
                <div>
                  <CheckCircle2 size={17} />
                  <span>
                    {t(
                      "Keine Meeting-Daten an Cloud-Anbieter",
                      "No meeting data sent to cloud providers",
                    )}
                  </span>
                </div>
              )}
            </div>
            {missing.length > 0 && (
              <div className="warning-box">
                <Cpu size={17} />
                <span>
                  {t("Modelle fehlen noch:", "Models still needed:")}{" "}
                  {missing.map((m) => m.model).join(", ")}
                  <small>
                    {t(
                      "Du kannst zunächst aufnehmen und die Modelle anschließend installieren. Live-Modi benötigen die Modelle vorher.",
                      "You can record first and install models afterwards. Live modes require models before recording.",
                    )}
                  </small>
                </span>
              </div>
            )}
            <p className="small muted">
              {t(
                "API-Kosten entstehen direkt beim gewählten Anbieter. Das Anfragenbudget begrenzt neue Aufrufe, nicht deren Preis.",
                "API charges are billed by your provider. The request budget limits new calls, not their price.",
              )}
            </p>
          </aside>
        </div>
        {confirmGemini && (
          <Modal className="confirm-modal" labelledBy="gemini-video-title" onClose={() => setConfirmGemini(false)}>
            <h2 id="gemini-video-title">{t("Gemini für dieses Meeting verwenden?", "Use Gemini for this meeting?")}</h2>
            <p>{t(
              "Beim Start der Auswertung sendet MeetingPal die ausgewählten Videoclips einschließlich Ton an Google Gemini. Dafür wird die Cloud-Verarbeitung für dieses Meeting aktiviert.",
              "When analysis starts, MeetingPal sends the selected video clips, including audio, to Google Gemini. This enables cloud processing for this meeting.",
            )}</p>
            <div className="field-row">
              <button className="secondary" onClick={() => setConfirmGemini(false)}>{t("Lokal bleiben", "Stay offline")}</button>
              <button className="primary" onClick={selectGeminiVideo}>{t("Gemini verwenden", "Use Gemini")}</button>
            </div>
          </Modal>
        )}
        <footer className="modal-footer">
          <label className="checkbox-row consent">
            <input
              type="checkbox"
              checked={c.consent}
              onChange={(e) => patch({ consent: e.target.checked })}
            />
            <span>
              {t(
                "Die erforderliche Einwilligung zur Aufnahme und Verarbeitung liegt vor.",
                "The required consent for recording and processing has been obtained.",
              )}
              <small>
                {t(
                  "MeetingPal informiert andere Teilnehmer nicht automatisch.",
                  "MeetingPal does not automatically notify other participants.",
                )}
              </small>
            </span>
          </label>
          <div className="footer-actions">
            {!file && (
              <button
                className="secondary"
                disabled={!valid || busy || !desktop}
                onClick={() =>
                  onStart(
                    {
                      ...c,
                      title: c.title || "Audio test",
                      mode: "post",
                      video: { kind: "none", id: null, crop: null },
                      videoAnalysis: "none",
                    },
                    true,
                  )
                }
              >
                {t("5-Sekunden-Audiotest", "5-second audio test")}
              </button>
            )}
            <button
              className="primary"
              disabled={!valid || busy || !desktop}
              onClick={() => onStart(c, false)}
            >
              {busy ? (
                <LoaderCircle className="spin" size={17} />
              ) : (
                <Mic size={17} />
              )}
              {file
                ? t("Importieren & auswerten", "Import & analyze")
                : t("Aufnahme starten", "Start recording")}
            </button>
          </div>
        </footer>
    </Modal>
  );
}

function ModelsPage({
  models,
  progress,
  t,
  act,
  refresh,
  notify,
}: {
  models: ModelSpec[];
  progress: Record<string, Progress>;
  t: Translate;
  act: (f: () => Promise<unknown>) => Promise<void>;
  refresh: () => Promise<void>;
  notify: (v: string) => void;
}) {
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">{t("LOKALE KI", "ON-DEVICE AI")}</div>
          <h1>
            {t(
              "Intelligenz, die bei dir bleibt.",
              "Intelligence that stays with you.",
            )}
          </h1>
          <p>
            {t(
              "Einmal herunterladen. Danach ohne Internet arbeiten.",
              "Download once. Work offline afterwards.",
            )}
          </p>
        </div>
        <span className="badge green">
          <ShieldCheck size={14} />
          {t("Keine zusätzliche KI-App nötig", "No separate AI app required")}
        </span>
      </div>
      <div className="info-box">
        <HardDrive size={19} />
        <span>
          {t(
            "Die Modelle werden separat von der App gespeichert. Jeder Download wird vor der Verwendung anhand seiner SHA-256-Prüfsumme verifiziert.",
            "Models are stored separately from the app. Every download is verified using its SHA-256 checksum before use.",
          )}
        </span>
      </div>
      <div className="model-grid">
        {models.map((m) => {
          const p = progress[m.id];
          const downloading = p?.state === "downloading";
          const total = m.files.reduce((s, f) => s + f.bytes, 0);
          return (
            <article className="model-card" key={m.id}>
              <div className="model-card-top">
                <span className="model-symbol">
                  {m.task === "transcription" ? (
                    <AudioLines />
                  ) : m.task === "vision" ? (
                    <Video />
                  ) : (
                    <Sparkles />
                  )}
                </span>
                <span className={`badge ${m.installed ? "green" : "neutral"}`}>
                  {m.installed ? t("Installiert", "Installed") : bytes(total)}
                </span>
              </div>
              <h3>{m.name}</h3>
              <p>
                {m.task === "transcription"
                  ? t(
                      "Sprache wird zu Text. Mehrsprachig und vollständig lokal.",
                      "Turn speech into text. Multilingual and entirely local.",
                    )
                  : m.task === "vision"
                    ? t(
                        "Präsentationen, Diagramme und Bildschirmbilder verstehen.",
                        "Understand presentations, diagrams and screen content.",
                      )
                    : t(
                        "Zusammenfassen, Aufgaben finden und Fragen beantworten.",
                        "Summarize, find tasks and answer questions.",
                      )}
              </p>
              <div className="model-meta">
                <span>{m.license}</span>
                <span>{bytes(total)}</span>
                <span>SHA-256</span>
              </div>
              {downloading && (
                <div className="download-status">
                  <progress value={p.received} max={p.total} />
                  <span>
                    {bytes(p.received)} / {bytes(p.total)}
                  </span>
                </div>
              )}
              {p?.error && <small className="error-text">{diagnostic(p.error, t)}</small>}
              <div className="model-actions">
                {downloading ? (
                  <button
                    className="secondary"
                    onClick={() =>
                      act(() => api("cancel_download", { id: m.id }))
                    }
                  >
                    <X size={16} />
                    {t("Abbrechen", "Cancel")}
                  </button>
                ) : m.installed ? (
                  <>
                    <button
                      className="secondary"
                      onClick={() =>
                        act(async () => {
                          if (m.task === "transcription") {
                            const b = await api<{
                              realtimeFactor: number;
                              processingSeconds: number;
                            }>("benchmark_model", { id: m.id });
                            notify(
                              `${t("Leistungstest", "Benchmark")}: ${b.processingSeconds.toFixed(1)} s · ${b.realtimeFactor.toFixed(2)}× ${t("Audiodauer (inkl. Modellstart)", "audio duration (including model loading)")}`,
                            );
                          } else
                            notify(
                              t(
                                "Das Modell ist für Auswertungen bereit.",
                                "The model is ready for analysis.",
                              ),
                            );
                        })
                      }
                    >
                      <Check size={16} />
                      {m.task === "transcription"
                        ? t("Leistung testen", "Run benchmark")
                        : t("Bereit zur Nutzung", "Ready to use")}
                    </button>
                    <button
                      className="icon-button"
                      aria-label={t("Modell löschen", "Delete model")}
                      onClick={() =>
                        act(async () => {
                          await api("delete_model", { id: m.id });
                          await refresh();
                        })
                      }
                    >
                      <Trash2 size={16} />
                    </button>
                  </>
                ) : (
                  <button
                    className="primary"
                    disabled={!desktop}
                    onClick={() => {
                      api("download_model", { id: m.id })
                        .then(refresh)
                        .catch((e) => notify(String(e)));
                    }}
                  >
                    <Download size={16} />
                    {t("Modell herunterladen", "Download model")}
                  </button>
                )}
              </div>
            </article>
          );
        })}
      </div>
    </>
  );
}

function SettingsPage({
  boot,
  updates,
  t,
  act,
  onSaved,
  notify,
}: {
  boot: Bootstrap;
  updates: ReturnType<typeof useUpdates>;
  t: Translate;
  act: (f: () => Promise<unknown>) => Promise<void>;
  onSaved: () => Promise<void>;
  notify: (v: string) => void;
}) {
  const [settings, setSettings] = useState<Settings>(() => ({
    ...structuredClone(boot.settings), defaultConfig: editableConfig(boot.settings.defaultConfig),
  }));
  const [keys, setKeys] = useState<Record<string, string>>({});
  const updateBusy = updates.status === "checking" || updates.status === "installing";
  let updateMessage = "";
  if (boot.activeMeeting) {
    updateMessage = t("Updates sind nach der Aufnahme verfügbar.", "Updates are available after recording.");
  } else if (updates.status === "checking") {
    updateMessage = t("Suche nach Updates …", "Checking for updates…");
  } else if (updates.status === "current") {
    updateMessage = t(`Version ${appVersion} ist aktuell.`, `Version ${appVersion} is up to date.`);
  } else if (updates.status === "installing") {
    updateMessage = t("Update wird installiert …", "Installing update…");
    if (updates.percent !== null) updateMessage += ` ${updates.percent} %`;
  }
  return (
    <>
      <div className="page-heading">
        <div>
          <div className="eyebrow">{t("DEIN ARBEITSBEREICH", "YOUR WORKSPACE")}</div>
          <h1>{t("So, wie du arbeitest.", "The way you work.")}</h1>
          <p>
            {t(
              "Anbieter verbinden, Standards festlegen, Daten im Blick behalten.",
              "Connect providers, set defaults and stay in control of your data.",
            )}
          </p>
        </div>
      </div>
      <div className="settings-layout">
        <section className="panel">
          <h2>
            <Globe size={20} />
            {t("Cloud-Anbieter", "Cloud providers")}
          </h2>
          <p className="muted">
            {t(
              "Schlüssel werden im Betriebssystem-Schlüsselspeicher gespeichert. Ist er nicht verfügbar, gelten sie nur für diese Sitzung.",
              "Keys are stored in your operating system credential store. If unavailable, they are kept only for this session.",
            )}
          </p>
          {(["openai", "anthropic", "gemini"] as Provider[]).map((provider) => (
            <div className="provider-setting" key={provider}>
              <div>
                <strong>{providerNames[provider]}</strong>
                <span
                  className={`badge ${boot.keys[provider] ? "green" : "neutral"}`}
                >
                  {boot.keys[provider]
                    ? t("Verbunden", "Connected")
                    : t("Nicht verbunden", "Not connected")}
                </span>
              </div>
              <div className="field-row">
                <input
                  type="password"
                  autoComplete="off"
                  aria-label={`${providerNames[provider]} ${t("API-Schlüssel", "API key")}`}
                  placeholder={t("API-Schlüssel", "API key")}
                  value={keys[provider] ?? ""}
                  onChange={(e) =>
                    setKeys({ ...keys, [provider]: e.target.value })
                  }
                />
                <button
                  className="secondary"
                  disabled={!keys[provider]}
                  onClick={() =>
                    act(async () => {
                      const stored = await api<boolean>("save_key", {
                        provider,
                        key: keys[provider],
                      });
                      cloudKeyChanged(provider);
                      setKeys((k) => ({ ...k, [provider]: "" }));
                      notify(
                        stored
                          ? t(
                              "Schlüssel sicher gespeichert.",
                              "Key stored securely.",
                            )
                          : t(
                              "Schlüssel nur für diese Sitzung gespeichert.",
                              "Key stored for this session only.",
                            ),
                      );
                      await onSaved();
                    })
                  }
                >
                  {t("Speichern", "Save")}
                </button>
                {boot.keys[provider] && (
                  <button
                    className="icon-button"
                    aria-label={t("Schlüssel entfernen", "Remove key")}
                    onClick={() =>
                      act(async () => {
                        await api("delete_key", { provider });
                        cloudKeyChanged(provider);
                        await onSaved();
                      })
                    }
                  >
                    <Trash2 size={16} />
                  </button>
                )}
              </div>
            </div>
          ))}
        </section>
        <section className="panel">
          <h2>
            <Settings2 size={20} />
            {t("Allgemein", "General")}
          </h2>
          <label className="field-label">
            {t("Sprache der Oberfläche", "Interface language")}
            <select
              value={settings.locale}
              onChange={(e) =>
                setSettings({ ...settings, locale: e.target.value })
              }
            >
              <option value="de">Deutsch</option>
              <option value="en">English</option>
            </select>
          </label>
          <label className="field-label">
            {t(
              "Aufbewahrung abgeschlossener Meetings",
              "Retention of completed meetings",
            )}
            <select
              value={settings.retentionDays ?? ""}
              onChange={(e) =>
                setSettings({
                  ...settings,
                  retentionDays: e.target.value ? Number(e.target.value) : null,
                })
              }
            >
              <option value="">
                {t("Bis ich sie selbst lösche", "Until I delete them")}
              </option>
              <option value="30">30 {t("Tage", "days")}</option>
              <option value="90">90 {t("Tage", "days")}</option>
              <option value="365">365 {t("Tage", "days")}</option>
            </select>
          </label>
          {settings.retentionDays && (
            <p className="warning-box">
              {t(
                "Ältere abgeschlossene Meetings werden beim nächsten App-Start einschließlich aller Aufnahmen gelöscht.",
                "Older completed meetings, including recordings, will be deleted on the next app launch.",
              )}
            </p>
          )}
          <h3>{t("Programmupdates", "App updates")}</h3>
          <label className="field-label">
            {t("Automatisch nach Updates suchen", "Automatically check for updates")}
            <select value={settings.updateChecks} onChange={(event) => setSettings({ ...settings, updateChecks: event.target.value as Settings["updateChecks"] })}>
              <option value="onlineOnly">{t("Bei Online-Profilen", "With online profiles")}</option>
              <option value="always">{t("Auch bei lokaler KI", "Also when using local AI")}</option>
              <option value="manual">{t("Nur manuell", "Manual only")}</option>
            </select>
          </label>
          <p className="muted">{t("Die Suche verbindet sich mit GitHub. Du bestimmst, wann ein Update installiert und die App neu gestartet wird.", "Checks connect to GitHub. You choose when to install an update and restart the app.")}</p>
          <div className="field-row">
            <button className="secondary" disabled={!desktop || !!boot.activeMeeting || updateBusy} onClick={() => void updates.check()}>
              <RefreshCw size={15} />{t("Jetzt nach Updates suchen", "Check for updates now")}
            </button>
            {updates.version && <button className="primary" disabled={!!boot.activeMeeting || updateBusy} onClick={() => void updates.install()}>
              <Download size={15} />{t(`Version ${updates.version} installieren und neu starten`, `Install ${updates.version} and restart`)}
            </button>}
          </div>
          {updateMessage && <p className="muted" role="status">{updateMessage}</p>}
          {updates.error && <p className="warning-box" role="alert">{updateError(updates.error, t)}</p>}
          <h3>{t("Standards für neue Meetings", "Defaults for new meetings")}</h3>
          <ModelPicker
            task="transcription"
            label={t("Transkription", "Transcription")}
            online={!settings.defaultConfig.offline}
            value={settings.defaultConfig.transcription}
            onChange={(transcription) =>
              setSettings(current => ({
                ...current,
                defaultConfig: withSelectedProviders({ ...current.defaultConfig, transcription }),
              }))
            }
            t={t}
          />
          <ModelPicker
            task="analysis"
            label={t("Auswertung", "Analysis")}
            online={!settings.defaultConfig.offline}
            value={settings.defaultConfig.analysis}
            onChange={(analysis) =>
              setSettings(current => ({
                ...current,
                defaultConfig: withSelectedProviders({ ...current.defaultConfig, analysis }),
              }))
            }
            t={t}
          />
          <button
            className="primary"
            onClick={() =>
              act(async () => {
                await api("save_settings", { settings });
                await onSaved();
                notify(t("Einstellungen gespeichert.", "Settings saved."));
              })
            }
          >
            <Check size={16} />
            {t("Einstellungen speichern", "Save settings")}
          </button>
        </section>
        <section className="panel">
          <h2>
            <FolderOpen size={20} />
            {t("Lokale Daten", "Local data")}
          </h2>
          <p>
            {t(
              "Aufnahmen und Ergebnisse sind normale, unverschlüsselte Dateien. Sichere dieses Verzeichnis regelmäßig.",
              "Recordings and results are ordinary, unencrypted files. Back up this folder regularly.",
            )}
          </p>
          <code className="path-display">{boot.dataDirectory}</code>
          <p className="muted">
            {t(
              "Keine Telemetrie. Kein MeetingPal-Konto. Keine automatische Cloud-Synchronisierung.",
              "No telemetry. No MeetingPal account. No automatic cloud synchronization.",
            )}
          </p>
          <h3>{t("Gebündelte Laufzeiten", "Bundled runtimes")}</h3>
          {Object.entries(boot.runtimes).map(([name, ready]) => (
            <div className="runtime-row" key={name}>
              <span>{name}</span>
              <span className={`badge ${ready ? "green" : "amber"}`}>
                {ready ? t("Bereit", "Ready") : t("Fehlt", "Missing")}
              </span>
            </div>
          ))}
        </section>
      </div>
    </>
  );
}

function Meter({
  label,
  value,
  icon,
}: {
  label: string;
  value: number;
  icon: React.ReactNode;
}) {
  return (
    <div className="meter">
      <span>
        {icon}
        {label}
      </span>
      <div className="meter-track">
        <div style={{ width: `${Math.min(100, Math.sqrt(value) * 100)}%` }} />
      </div>
    </div>
  );
}
function MeetingPage({
  meeting: m,
  levels,
  t,
  busy,
  act,
  update,
  onBack,
  onDeleted,
}: {
  meeting: Meeting;
  levels: Levels;
  t: Translate;
  busy: boolean;
  act: (f: () => Promise<unknown>) => Promise<void>;
  update: (m: Meeting) => void;
  onBack: () => void;
  onDeleted: () => void;
}) {
  const active = ["starting", "recording", "paused"].includes(m.status);
  const [tab, setTab] = useState("overview");
  const [query, setQuery] = useState("");
  const [draft, setDraft] = useState("");
  useEffect(() => {
    setDraft("");
    if (!desktop || !m.config.streamTranscription) return;
    let active = true;
    const subscription = listen<{meetingId: string; text: string}>("transcript-preview", ({payload}) => {
      if (active && payload.meetingId === m.id) setDraft(payload.text);
    });
    return () => { active = false; void subscription.then((unlisten) => unlisten()); };
  }, [m.id, m.config.streamTranscription]);
  const [question, setQuestion] = useState("");
  const [edit, setEdit] = useState<TranscriptSegment | null>(null);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [notes, setNotes] = useState(m.notes);
  const [title, setTitle] = useState(m.config.title);
  const [tags, setTags] = useState(m.tags.join(", "));
  const [mediaUrl, setMediaUrl] = useState("");
  const [video, setVideo] = useState(false);
  const [deleteChoice, setDeleteChoice] = useState<boolean | null>(null);
  const [budget, setBudget] = useState(m.config.maxRequests);
  const [limit, setLimit] = useState(100);
  const player = useRef<HTMLMediaElement | null>(null);
  const pendingSeek = useRef(0);
  const pendingPlay = useRef(false);
  useEffect(() => {
    setNotes(m.notes);
    setTitle(m.config.title);
    setTags(m.tags.join(", "));
  }, [m.id]);
  useEffect(() => {
    api<Job[]>("get_jobs", { id: m.id })
      .then(setJobs)
      .catch(() => {});
  }, [m.id, m.updatedAt]);
  useEffect(() => {
    if (m.playbackFile)
      api<string>("media_path", { id: m.id, file: m.playbackFile })
        .then((path) => {
          setVideo(m.playbackFile!.endsWith(".webm"));
          setMediaUrl(convertFileSrc(path));
        })
        .catch(() => {});
  }, [m.id, m.playbackFile]);
  const seek = (atMs: number) =>
    act(async () => {
      let file = m.playbackFile;
      let offset = atMs;
      if (!file) {
        const segment = m.media.find(
          (s) => s.startMs <= atMs && s.endMs > atMs,
        );
        if (!segment) return;
        file = segment.mixed;
        offset -= segment.startMs;
      }
      const path = await api<string>("media_path", { id: m.id, file });
      const url = convertFileSrc(path);
      pendingSeek.current = offset / 1000;
      pendingPlay.current = true;
      if (url === mediaUrl && player.current) {
        player.current.currentTime = pendingSeek.current;
        await player.current.play();
      } else {
        setVideo(file.endsWith(".webm"));
        setMediaUrl(url);
      }
    });
  const sourceClick = (id: string) => {
    const segment = m.transcript.find((s) => s.id === id);
    const visual = m.visuals.find((v) => v.id === id);
    if (segment || visual) seek(segment?.startMs ?? visual!.atMs);
  };
  const evidenceList = (items: EvidenceItem[]) =>
    items.length ? (
      items.map((item, i) => (
        <li key={i}>
          <span>{item.text}</span>
          {item.owner && (
            <small className="owner">
              {item.owner}
              {item.dueDate ? ` · ${item.dueDate}` : ""}
            </small>
          )}
          <div className="evidence-links">
            {item.sources.map((id) => (
              <button key={id} onClick={() => sourceClick(id)}>
                {duration(
                  m.transcript.find((s) => s.id === id)?.startMs ??
                    m.visuals.find((v) => v.id === id)?.atMs ??
                    0,
                )}
                <ArrowUpRight size={10} />
              </button>
            ))}
          </div>
        </li>
      ))
    ) : (
      <li className="muted small">
        {t("Bisher nichts festgehalten.", "Nothing captured yet.")}
      </li>
    );
  const exported = (format: string) =>
    act(async () => {
      const path = await save({
        defaultPath: `${m.config.title.replace(/[<>:"/\\|?*]/g, "_")}.${format}`,
        filters: [{ name: format.toUpperCase(), extensions: [format] }],
      });
      if (path) await api("export_meeting", { id: m.id, path, format });
    });
  const transcript = m.transcript.filter((s) =>
    `${s.text} ${s.speaker ?? ""}`.toLowerCase().includes(query.toLowerCase()),
  );
  return (
    <>
      <button className="text-button back" onClick={onBack}>
        <ArrowLeft size={15} />
        {t("Alle Meetings", "All meetings")}
      </button>
      <div className="page-heading detail-heading">
        <div>
          <div className="eyebrow">
            {new Date(m.createdAt).toLocaleString(t("de-DE", "en-US"))}
          </div>
          <h1>{m.config.title}</h1>
          <div className="detail-meta">
            <span>
              <Clock3 size={14} />
              {duration(active ? levels.duration_ms : m.durationMs)}
            </span>
            <span>
              <ShieldCheck size={14} />
              {m.config.offline
                ? t("Lokales Profil", "Local profile")
                : t("Cloud / gemischt", "Cloud / mixed")}
            </span>
            <span className={`badge ${active ? "recording-badge" : "neutral"}`}>
              {statusLabel(m.status, t)}
            </span>
          </div>
        </div>
        <div className="field-row">
          <select
            aria-label={t("Exportformat", "Export format")}
            defaultValue=""
            onChange={(e) => {
              exported(e.target.value);
              e.target.value = "";
            }}
          >
            <option value="" disabled>
              {t("Exportieren …", "Export …")}
            </option>
            {["md", "pdf", "json", "srt", "vtt"].map((f) => (
              <option key={f}>{f}</option>
            ))}
          </select>
          <button
            className="icon-button"
            title={t("Meeting löschen", "Delete meeting")}
            disabled={active}
            onClick={() => setDeleteChoice(false)}
          >
            <Trash2 size={18} />
          </button>
        </div>
      </div>
      {active && (
        <div className="recording-bar">
          <div className="recording-timer">
            <span
              className={m.status === "paused" ? "paused-dot" : "record-dot"}
            />
            {duration(levels.duration_ms)}
          </div>
          <Meter
            label={t("Mikrofon", "Microphone")}
            value={levels.microphone}
            icon={<Mic size={14} />}
          />
          <Meter
            label={t("Desktop", "Desktop")}
            value={levels.desktop}
            icon={<Monitor size={14} />}
          />
          <button
            className="secondary"
            title="Ctrl/Cmd + Shift + Space"
            onClick={() =>
              act(() =>
                api("control_recording", {
                  action: m.status === "paused" ? "resume" : "pause",
                }),
              )
            }
          >
            {m.status === "paused" ? <Play size={16} /> : <Pause size={16} />}
          </button>
          <button
            className="secondary"
            title={t("Zeitmarke setzen", "Add bookmark")}
            onClick={() =>
              act(async () =>
                update(
                  await api<Meeting>("add_marker", {
                    id: m.id,
                    atMs: levels.duration_ms,
                    text: t("Zeitmarke", "Bookmark"),
                  }),
                ),
              )
            }
          >
            <Bookmark size={16} />
          </button>
          <button
            className="danger"
            onClick={() =>
              act(() => api("control_recording", { action: "stop" }))
            }
          >
            <Square size={14} />
            {t("Beenden", "Stop")}
          </button>
        </div>
      )}
      {m.warnings.length > 0 && (
        <details className="warning-box warning-details">
          <summary>
            <AlertCircle size={16} />
            {t(
              "Hinweise zur Aufnahme und Verarbeitung",
              "Recording and processing notices",
            )}{" "}
            ({m.warnings.length})
          </summary>
          <ul>
            {m.warnings.map((warning, i) => (
              <li key={i}>{diagnostic(warning, t)}</li>
            ))}
          </ul>
        </details>
      )}
      {m.status === "needs_attention" ||
      m.status === "recovered" ||
      jobs.some((job) => job.state === "failed" || job.state === "paused") ||
      (m.status === "recorded" && !m.tags.includes("audio-test")) ? (
        <div className="recovery-bar">
          <span>
            {active
              ? t("Die KI-Verarbeitung wartet. Du kannst sie fortsetzen; die Aufnahme bleibt davon unabhängig.", "AI processing is waiting. You can resume it independently of the recording.")
              : t("Die Aufnahme ist lokal gespeichert. Du kannst die Verarbeitung fortsetzen.", "Your recording is saved locally. You can resume processing.")}
          </span>
          <input
            type="number"
            min={1}
            max={100000}
            aria-label={t("Anfragelimit", "Request budget")}
            value={budget}
            onChange={(e) => setBudget(Number(e.target.value))}
          />
          <button
            className="secondary"
            onClick={() =>
              act(() =>
                api("retry_processing", { id: m.id, maxRequests: budget }),
              )
            }
          >
            <RefreshCw size={15} />
            {t("Fortsetzen", "Resume")}
          </button>
        </div>
      ) : null}
      {mediaUrl && (
        <div className="player-panel">
          {video ? (
            <video
              ref={(el) => {
                player.current = el;
              }}
              src={mediaUrl}
              controls
              onLoadedMetadata={() => {
                if (player.current) {
                  player.current.currentTime = pendingSeek.current;
                  if (pendingPlay.current) {
                    pendingPlay.current = false;
                    player.current.play().catch(() => {});
                  }
                }
              }}
            />
          ) : (
            <audio
              ref={(el) => {
                player.current = el;
              }}
              src={mediaUrl}
              controls
              onLoadedMetadata={() => {
                if (player.current) {
                  player.current.currentTime = pendingSeek.current;
                  if (pendingPlay.current) {
                    pendingPlay.current = false;
                    player.current.play().catch(() => {});
                  }
                }
              }}
            />
          )}
        </div>
      )}
      <div className="tabs">
        {[
          ["overview", t("Überblick", "Overview")],
          ["transcript", t("Transkript", "Transcript")],
          ["notes", t("Eigene Notizen", "Your notes")],
          ["assistant", t("Meeting fragen", "Ask this meeting")],
          ["processing", t("Verarbeitung", "Processing")],
        ].map(([id, label]) => (
          <button
            className={tab === id ? "active" : ""}
            key={id}
            aria-pressed={tab === id}
            onClick={() => setTab(id)}
          >
            {label}
            {id === "transcript" && m.transcript.length > 0 && (
              <span>{m.transcript.length}</span>
            )}
          </button>
        ))}
      </div>
      {tab === "overview" &&
        (m.analysis ? (
          <>
            <div className="result-heading">
              <span className="muted small">
                {m.analysis.provisional
                  ? t(
                      "Vorläufiger KI-Entwurf · bitte überprüfen",
                      "Provisional AI draft · please review",
                    )
                  : t(
                      "KI-Entwurf · Quellen über die Zeitmarken prüfen",
                      "AI draft · verify sources using timestamps",
                    )}
              </span>
              <button
                className="text-button"
                disabled={busy || active}
                onClick={() =>
                  act(async () =>
                    update(await api<Meeting>("analyze_again", { id: m.id })),
                  )
                }
              >
                <RefreshCw size={14} />
                {t("Neu auswerten", "Analyze again")}
              </button>
            </div>
            <div className="results-grid">
              {[
                [
                  t("Zusammenfassung", "Summary"),
                  m.analysis.summary,
                  <Sparkles size={19} />,
                ],
                [
                  t("Entscheidungen", "Decisions"),
                  m.analysis.decisions,
                  <CheckCircle2 size={19} />,
                ],
                [
                  t("Aufgaben & nächste Schritte", "Tasks & next steps"),
                  m.analysis.tasks,
                  <Check size={19} />,
                ],
                [
                  t("Offene Fragen", "Open questions"),
                  m.analysis.questions,
                  <FileText size={19} />,
                ],
                [
                  t("Themen", "Topics"),
                  m.analysis.topics,
                  <Library size={19} />,
                ],
              ].map(([title, items, icon], i) => (
                <section
                  className={`panel result-panel ${i === 0 ? "wide" : ""}`}
                  key={i}
                >
                  <h2>
                    {icon as React.ReactNode}
                    {title as string}
                  </h2>
                  <ul className="result-list">
                    {evidenceList(items as EvidenceItem[])}
                  </ul>
                </section>
              ))}
            </div>
          </>
        ) : (
          <div className="empty-state">
            <span className="empty-icon">
              <Sparkles size={29} />
            </span>
            <h3>
              {active
                ? t(
                    "Du sprichst. MeetingPal hört zu.",
                    "You speak. MeetingPal listens.",
                  )
                : t(
                    "Hier entstehen deine Ergebnisse.",
                    "Your results will appear here.",
                  )}
            </h3>
            <p>
              {t(
                "Zusammenfassung, Entscheidungen und Aufgaben erscheinen nach der Auswertung.",
                "Your summary, decisions and tasks appear after analysis.",
              )}
            </p>
          </div>
        ))}
      {tab === "transcript" && (
        <>
          {draft && <p className="transcript-draft"><strong>{t("Vorläufig", "Provisional")}</strong> {draft}</p>}
          <div className="section-heading">
            <p className="muted small">
              {t(
                "Text anklicken, um ihn zu korrigieren. Zeitmarken spielen die Aufnahme ab.",
                "Click text to edit it. Timestamps play the recording.",
              )}
            </p>
            <label className="search-box">
              <Search size={15} />
              <input
                aria-label={t("Transkript durchsuchen", "Search transcript")}
                placeholder={t(
                  "Transkript durchsuchen …",
                  "Search transcript …",
                )}
                value={query}
                onChange={(e) => {
                  setQuery(e.target.value);
                  setLimit(100);
                }}
              />
            </label>
          </div>
          <div className="transcript-list">
            {transcript.slice(0, limit).map((s) => (
              <div className="transcript-row" key={s.id}>
                <button className="timestamp" onClick={() => seek(s.startMs)}>
                  {duration(s.startMs)}
                  <Play size={10} />
                </button>
                <div>
                  {edit?.id === s.id ? (
                    <div className="transcript-edit">
                      <input
                        aria-label={t("Sprechername", "Speaker name")}
                        value={edit.speaker ?? ""}
                        placeholder={t(
                          "Sprechername (optional)",
                          "Speaker name (optional)",
                        )}
                        onChange={(e) =>
                          setEdit({ ...edit, speaker: e.target.value || null })
                        }
                      />
                      <textarea
                        aria-label={t("Transkripttext", "Transcript text")}
                        value={edit.text}
                        onChange={(e) =>
                          setEdit({ ...edit, text: e.target.value })
                        }
                      />
                      <div className="field-row">
                        <button
                          className="primary"
                          onClick={() =>
                            act(async () => {
                              update(
                                await api<Meeting>("edit_transcript", {
                                  id: m.id,
                                  segmentId: edit.id,
                                  text: edit.text,
                                  speaker: edit.speaker,
                                }),
                              );
                              setEdit(null);
                            })
                          }
                        >
                          {t("Speichern", "Save")}
                        </button>
                        <button
                          className="secondary"
                          onClick={() => setEdit(null)}
                        >
                          {t("Abbrechen", "Cancel")}
                        </button>
                      </div>
                    </div>
                  ) : (
                    <button
                      className="transcript-text"
                      onClick={() => setEdit({ ...s })}
                    >
                      <strong>
                        {s.speaker ??
                          t("Sprecher nicht zugeordnet", "Unassigned speaker")}
                        {s.provisional && (
                          <small>{t("vorläufig", "provisional")}</small>
                        )}
                        {s.edited && <small>{t("bearbeitet", "edited")}</small>}
                      </strong>
                      <p>{s.text}</p>
                    </button>
                  )}
                </div>
              </div>
            ))}
            {!transcript.length && (
              <div className="empty-state">
                <AudioLines size={30} />
                <p>
                  {t("Noch kein Transkript vorhanden.", "No transcript yet.")}
                </p>
              </div>
            )}
          </div>
          {transcript.length > limit && (
            <button className="secondary" onClick={() => setLimit(limit + 100)}>
              {t("Weitere Abschnitte anzeigen", "Show more segments")}
            </button>
          )}
        </>
      )}
      {tab === "notes" && (
        <section className="panel notes-panel">
          <div className="two-columns">
            <label className="field-label">
              {t("Titel", "Title")}
              <input value={title} onChange={(e) => setTitle(e.target.value)} />
            </label>
            <label className="field-label">
              Tags
              <input
                value={tags}
                onChange={(e) => setTags(e.target.value)}
                placeholder={t("Projekt, Planung", "Project, planning")}
              />
            </label>
          </div>
          <label className="field-label">
            {t("Deine Notizen", "Your notes")}
            <textarea
              rows={12}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder={t(
                "Gedanken, Kontext und wichtige Punkte …",
                "Thoughts, context and important points …",
              )}
            />
          </label>
          <button
            className="primary"
            onClick={() =>
              act(async () =>
                update(
                  await api<Meeting>("update_notes", {
                    id: m.id,
                    title,
                    tags: tags
                      .split(",")
                      .map((v) => v.trim())
                      .filter(Boolean),
                    notes,
                  }),
                ),
              )
            }
          >
            <Check size={16} />
            {t("Notizen speichern", "Save notes")}
          </button>
          {m.markers.length > 0 && (
            <>
              <h3>{t("Zeitmarken", "Bookmarks")}</h3>
              {m.markers.map((marker, i) => (
                <button
                  className="marker"
                  key={i}
                  onClick={() => seek(marker.atMs)}
                >
                  <Bookmark size={14} />
                  {duration(marker.atMs)} · {marker.text}
                </button>
              ))}
            </>
          )}
        </section>
      )}
      {tab === "assistant" && (
        <section className="panel chat-panel">
          <div className="chat-intro">
            <span className="empty-icon">
              <Sparkles size={25} />
            </span>
            <h2>
              {t(
                "Was möchtest du nachschlagen?",
                "What would you like to look up?",
              )}
            </h2>
            <p>
              {t(
                "Antworten beziehen sich auf dieses Meeting und verlinken ihre Quellen.",
                "Answers refer to this meeting and link to their sources.",
              )}
            </p>
          </div>
          {m.answers.map((a, i) => (
            <div className="chat-pair" key={i}>
              <div className="chat-question">{a.question}</div>
              <div className="chat-answer">
                <Sparkles size={18} />
                <ul className="result-list">{evidenceList([a.answer])}</ul>
              </div>
            </div>
          ))}
          <form
            className="chat-input"
            onSubmit={(e) => {
              e.preventDefault();
              act(async () => {
                update(
                  await api<Meeting>("ask_meeting", { id: m.id, question }),
                );
                setQuestion("");
              });
            }}
          >
            <input
              value={question}
              maxLength={2000}
              onChange={(e) => setQuestion(e.target.value)}
              placeholder={t(
                "Welche nächsten Schritte wurden vereinbart?",
                "What next steps did we agree on?",
              )}
            />
            <button
              className="primary"
              disabled={busy || !question.trim() || !m.transcript.length}
              aria-label={t("Frage senden", "Send question")}
            >
              {busy ? (
                <LoaderCircle size={19} className="spin" />
              ) : (
                <Send size={19} />
              )}
            </button>
          </form>
        </section>
      )}
      {tab === "processing" && (
        <section className="panel">
          <div className="stats-row">
            <Stat
              icon={<Globe size={19} />}
              label={t("Cloud-Anfragen / Budget", "Cloud requests / budget")}
              value={`${m.cloudRequests} / ${m.config.maxRequests}`}
            />
            <Stat
              icon={<Cpu size={19} />}
              label={t("Gemeldete Tokens", "Reported tokens")}
              value={m.tokens.toLocaleString(t("de-DE", "en-US"))}
            />
            <Stat
              icon={<Check size={19} />}
              label={t("Abgeschlossene Aufträge", "Completed jobs")}
              value={`${jobs.filter((j) => j.state === "done").length} / ${jobs.length}`}
            />
          </div>
          <p className="muted small">
            {t(
              "Live-Text entsteht abschnittsweise. Ein Rückstand der KI beeinflusst die Aufnahme nicht.",
              "Live text is processed in segments. AI processing lag does not affect recording.",
            )}
          </p>
          <div className="job-list">
            {jobs.slice(-100).map((j) => (
              <div className="job-row" key={j.id}>
                <span>
                  {statusLabel(j.kind, t)}
                  <small>{j.payload}</small>
                </span>
                <span
                  className={`badge ${j.state === "done" ? "green" : j.state === "failed" ? "amber" : "neutral"}`}
                >
                  {statusLabel(j.state, t)}
                </span>
                {j.error && <small className="error-text">{diagnostic(j.error, t)}</small>}
              </div>
            ))}
          </div>
          {m.media.length > 0 && !active && (
            <div className="field-row">
              <button
                className="secondary"
                onClick={() =>
                  act(async () => {
                    await api("make_playback", { id: m.id });
                    update(await api<Meeting>("get_meeting", { id: m.id }));
                  })
                }
              >
                <Play size={16} />
                {t("Wiedergabe vorbereiten", "Prepare playback")}
              </button>
              <button
                className="secondary"
                onClick={() => setDeleteChoice(true)}
              >
                <Trash2 size={16} />
                {t("Nur Mediendateien löschen", "Delete media files only")}
              </button>
            </div>
          )}
        </section>
      )}
      {deleteChoice !== null && (
        <Modal className="confirm-modal" labelledBy="delete-title" onClose={() => setDeleteChoice(null)}>
            <Trash2 size={28} />
            <h2 id="delete-title">
              {deleteChoice
                ? t("Mediendateien löschen?", "Delete media files?")
                : t(
                    "Meeting endgültig löschen?",
                    "Permanently delete this meeting?",
                  )}
            </h2>
            <p>
              {deleteChoice
                ? t(
                    "Transkript, Notizen und Auswertung bleiben erhalten.",
                    "The transcript, notes and analysis will be kept.",
                  )
                : t(
                    "Die Aufnahme, das Transkript und alle Ergebnisse werden von diesem Gerät entfernt.",
                    "The recording, transcript and all results will be removed from this device.",
                  )}
            </p>
            <div className="field-row">
              <button
                className="secondary"
                onClick={() => setDeleteChoice(null)}
              >
                {t("Abbrechen", "Cancel")}
              </button>
              <button
                className="danger"
                onClick={() =>
                  act(async () => {
                    await api("delete_meeting", {
                      id: m.id,
                      mediaOnly: deleteChoice,
                    });
                    if (deleteChoice) {
                      update(await api<Meeting>("get_meeting", { id: m.id }));
                      setMediaUrl("");
                    } else onDeleted();
                    setDeleteChoice(null);
                  })
                }
              >
                {t("Löschen", "Delete")}
              </button>
            </div>
        </Modal>
      )}
    </>
  );
}
export default App;
