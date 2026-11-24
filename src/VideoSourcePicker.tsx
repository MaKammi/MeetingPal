import { useEffect, useRef, useState } from "react";
import {
  AppWindow,
  Check,
  CheckCircle2,
  ChevronRight,
  ImageOff,
  LoaderCircle,
  Maximize2,
  Monitor,
  RefreshCw,
  Scan,
  Video,
  X,
} from "lucide-react";
import { api } from "./api";
import { CropEditor, dimensions, fits } from "./CropEditor";
import { diagnostic } from "./diagnostics";
import type {
  Crop,
  MeetingConfig,
  Source,
  SourcePreview,
  Translate,
} from "./types";
import "./video-source.css";

const brandIcon = new URL("../resources/icon.svg", import.meta.url).href;
type VideoSelection = MeetingConfig["video"];
type Choice = { source: Source; name: string; frame?: SourcePreview };
type Page = "list" | "preview" | "crop";
type Frames = Record<string, SourcePreview | null | undefined>;
const keyOf = (s: Source) => s.kind + ":" + s.id;
const isGrant = (id?: string | null) => !!id?.startsWith("portal:");
const releaseGrant = (id: string) => {
  void api("release_portal_source", { id }).catch(() => {});
};

function sourceName(source: Source, sources: Source[], t: Translate) {
  if (source.kind === "window")
    return source.name || t("Freigegebenes Fenster", "Shared window");
  if (isGrant(source.id)) return t("Freigegebener Bildschirm", "Shared screen");
  const index = sources
    .filter((s) => s.kind === "screen")
    .findIndex((s) => s.id === source.id);
  return t("Bildschirm ", "Screen ") + (index + 1);
}
function description(source: Source, crop: Crop | null, t: Translate) {
  return crop
    ? t("Ausschnitt", "Region") + " · " + dimensions(crop)
    : source.kind === "window"
      ? t("Gesamtes Fenster", "Entire window")
      : t("Gesamter Bildschirm", "Entire screen");
}
function Picture({
  frame,
  crop,
  label,
}: {
  frame: SourcePreview;
  crop: Crop | null;
  label: string;
}) {
  if (!crop || !fits(crop, frame))
    return (
      <img
        className="source-picture"
        src={frame.image}
        alt={label}
        draggable={false}
      />
    );
  // The viewBox uses the same physical pixels as the recorder, even for a thumbnail.
  return (
    <svg
      className="source-picture"
      role="img"
      aria-label={label}
      viewBox={[crop.x, crop.y, crop.width, crop.height].join(" ")}
    >
      <svg
        x={crop.x}
        y={crop.y}
        width={crop.width}
        height={crop.height}
        viewBox={[crop.x, crop.y, crop.width, crop.height].join(" ")}
        overflow="hidden"
      >
        <image
          href={frame.image}
          width={frame.width}
          height={frame.height}
          preserveAspectRatio="none"
        />
      </svg>
    </svg>
  );
}

export function VideoSourceField({
  value,
  onChange,
  sources,
  t,
}: {
  value: VideoSelection;
  onChange: (v: VideoSelection) => void;
  sources: Source[];
  t: Translate;
}) {
  const [open, setOpen] = useState(false);
  const [confirmed, setConfirmed] = useState<Choice | null>(null);
  const known = sources.find(
    (s) =>
      s.id === value.id &&
      s.kind === (value.kind === "region" ? "screen" : value.kind),
  );
  const current = confirmed?.source.id === value.id ? confirmed : null;
  const name = current?.name ?? (known && sourceName(known, sources, t));
  const source = current?.source ?? known;
  useEffect(() => {
    const id = value.id;
    return () => {
      if (id && isGrant(id)) releaseGrant(id);
    };
  }, [value.id]);
  return (
    <div className="video-source-field">
      {current?.frame ? (
        <div className="source-field-image">
          <Picture
            frame={current.frame}
            crop={value.crop}
            label={t("Vorschau der Aufnahmequelle", "Recording source preview")}
          />
        </div>
      ) : (
        <span className="source-field-icon">
          <Video />
        </span>
      )}
      <div className="source-field-label">
        <strong>{name || t("Aufnahmequelle", "Recording source")}</strong>
        <small>
          {source
            ? description(source, value.crop, t)
            : t(
                "Wähle einen Bildschirm oder ein Fenster.",
                "Choose a screen or a window.",
              )}
        </small>
      </div>
      <button
        type="button"
        className="source-button"
        onClick={() => setOpen(true)}
      >
        {value.id
          ? t("Quelle ändern", "Change source")
          : t("Quelle auswählen", "Choose source")}
      </button>
      {open && (
        <SourceDialog
          value={value}
          confirmed={current}
          onClose={() => setOpen(false)}
          t={t}
          onApply={(choice, crop) => {
            setConfirmed(choice);
            const { source } = choice;
            onChange({
              kind: isGrant(source.id)
                ? "portal"
                : crop
                  ? "region"
                  : source.kind,
              id: source.id,
              crop,
            });
            setOpen(false);
          }}
        />
      )}
    </div>
  );
}

function SourceDialog({
  value,
  confirmed,
  onClose,
  onApply,
  t,
}: {
  value: VideoSelection;
  confirmed: Choice | null;
  onClose: () => void;
  onApply: (choice: Choice, crop: Crop | null) => void;
  t: Translate;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const heading = useRef<HTMLHeadingElement>(null);
  const [page, setPage] = useState<Page>("list");
  const [cropBack, setCropBack] = useState<Page>("list");
  const [sources, setSources] = useState<Source[]>([]);
  const [grant, setGrant] = useState<Choice | null>(
    isGrant(confirmed?.source.id) ? confirmed : null,
  );
  const [portalBusy, setPortalBusy] = useState(false);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  const [frames, setFrames] = useState<Frames>({});
  const [detail, setDetail] = useState<SourcePreview | null | undefined>();
  const [selectedKey, setSelectedKey] = useState<string | null>(
    confirmed
      ? keyOf(confirmed.source)
      : value.id
        ? (value.kind === "region" ? "screen" : value.kind) + ":" + value.id
        : null,
  );
  const [crop, setCrop] = useState<Crop | null>(value.crop);
  const draftGrant = useRef<string | null>(null);
  const mounted = useRef(false);
  useEffect(() => {
    const element = dialog.current!;
    mounted.current = true;
    element.showModal();
    return () => {
      mounted.current = false;
      element.close();
      if (draftGrant.current) releaseGrant(draftGrant.current);
    };
  }, []);
  useEffect(() => {
    heading.current?.focus();
  }, [page]);
  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      setError("");
      setFrames({});
      setSources([]);
      try {
        const list = await api<Source[]>("list_video_sources");
        if (cancelled) return;
        setSources(list);
        const available = list.filter((s) => s.kind !== "portal");
        const first =
          available.find((s) => s.isPrimary) ??
          available.find((s) => s.kind === "screen") ??
          available[0];
        setSelectedKey((prev) => prev ?? (first ? keyOf(first) : null));
        setLoading(false);
        let index = 0;
        async function thumbnails() {
          while (!cancelled && index < available.length) {
            const s = available[index++];
            const frame = await api<SourcePreview>("capture_source_preview", {
              kind: s.kind,
              id: s.id,
              detail: false,
            }).catch(() => null);
            if (!cancelled)
              setFrames((prev) => ({ ...prev, [keyOf(s)]: frame }));
          }
        }
        await Promise.all([thumbnails(), thumbnails()]);
      } catch (e) {
        if (!cancelled) {
          setError(String(e));
          setLoading(false);
        }
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [revision]);

  const display = [
    ...sources.filter((s) => s.kind !== "portal"),
    ...(grant ? [grant.source] : []),
  ];
  const selected = display.find((s) => keyOf(s) === selectedKey);
  const name = selected ? sourceName(selected, display, t) : "";
  const tileFrame =
    selected && (isGrant(selected.id) ? grant?.frame : frames[keyOf(selected)]);
  const frame = detail || tileFrame || undefined;
  const cropFits = !crop || !frame || fits(crop, frame);
  const canCrop =
    selected?.kind === "screen" &&
    !!frame &&
    frame.width >= 16 &&
    frame.height >= 16;

  useEffect(() => {
    if (page === "list" || !selected || isGrant(selected.id)) return;
    let cancelled = false;
    api<SourcePreview>("capture_source_preview", {
      kind: selected.kind,
      id: selected.id,
      detail: true,
    })
      .then((image) => {
        if (!cancelled) setDetail(image);
      })
      .catch(() => {
        if (!cancelled) setDetail(null);
      });
    return () => {
      cancelled = true;
    };
  }, [selected, page]);
  function choose(source: Source) {
    if (keyOf(source) === selectedKey) return;
    setSelectedKey(keyOf(source));
    setCrop(null);
    setDetail(undefined);
  }
  async function pickPortal() {
    setPortalBusy(true);
    setError("");
    try {
      const { source, preview } = await api<{
        source: Source;
        preview: SourcePreview | null;
      }>("select_portal_source");
      if (!mounted.current) {
        releaseGrant(source.id);
        return;
      }
      if (draftGrant.current) releaseGrant(draftGrant.current);
      draftGrant.current = source.id;
      setGrant({
        source,
        name: sourceName(source, [source], t),
        frame: preview ?? undefined,
      });
      choose(source);
    } catch (e) {
      if (mounted.current) setError(String(e));
    } finally {
      if (mounted.current) setPortalBusy(false);
    }
  }
  function apply(region: Crop | null) {
    if (!selected) return;
    if (draftGrant.current === selected.id) draftGrant.current = null;
    onApply({ source: selected, name, frame }, region);
  }
  function back() {
    if (page === "list") onClose();
    else {
      setPage(page === "crop" ? cropBack : "list");
      setDetail(undefined);
    }
  }
  function openPage(next: "preview" | "crop") {
    if (next === "crop") setCropBack(page);
    setDetail(undefined);
    setPage(next);
  }
  const waiting = (
    <span className="source-placeholder" role="status">
      <LoaderCircle className="spin" />
      {t("Vorschau wird geladen …", "Loading preview…")}
    </span>
  );
  const unavailable = (
    <span className="source-placeholder">
      <ImageOff />
      {t("Vorschau nicht verfügbar", "Preview unavailable")}
    </span>
  );
  function group(kind: "screen" | "window") {
    const members = display.filter((s) => s.kind === kind);
    return (
      <section
        className={"source-group " + kind}
        aria-labelledby={"sources-" + kind}
      >
        <h3 id={"sources-" + kind}>
          {kind === "screen" ? <Monitor /> : <AppWindow />}
          {kind === "screen"
            ? t("Bildschirme", "Screens")
            : t("Fenster", "Windows")}
        </h3>
        <div className="source-grid">
          {members.map((source) => {
            const key = keyOf(source),
              label = sourceName(source, display, t);
            const preview = isGrant(source.id)
              ? (grant?.frame ?? null)
              : frames[key];
            const size = preview ?? source;
            const caption = [
              source.isPrimary ? t("Hauptbildschirm", "Primary screen") : "",
              size.width && size.height ? dimensions(size) : "",
            ]
              .filter(Boolean)
              .join(" · ");
            return (
              <label key={key} className="source-option">
                <input
                  type="radio"
                  name="recording-source"
                  aria-label={label}
                  checked={selectedKey === key}
                  onChange={() => choose(source)}
                />
                <span className="source-tile">
                  <span className="source-thumb">
                    {preview ? (
                      <img
                        src={preview.image}
                        alt={t("Vorschau von ", "Preview of ") + label}
                        draggable={false}
                      />
                    ) : preview === null ? (
                      unavailable
                    ) : (
                      waiting
                    )}
                  </span>
                  <span className="source-caption">
                    <strong title={source.name || label}>{label}</strong>
                    <small>{caption}</small>
                  </span>
                  {key === selectedKey && (
                    <span className="source-check">
                      <Check size={14} />
                    </span>
                  )}
                </span>
              </label>
            );
          })}
        </div>
        {!members.length && (
          <p className="source-empty">
            {loading
              ? t("Quellen werden geladen …", "Loading sources…")
              : kind === "screen"
                ? t("Keine Bildschirme verfügbar.", "No screens available.")
                : t("Keine Fenster verfügbar.", "No windows available.")}
          </p>
        )}
      </section>
    );
  }
  const editButton = selected?.kind === "screen" && (
    <button
      type="button"
      className="source-button"
      disabled={!canCrop}
      onClick={() => openPage("crop")}
    >
      <Scan />
      {crop
        ? t("Bereich ändern", "Edit region")
        : t("Bereich aufziehen", "Draw region")}
    </button>
  );
  return (
    <dialog
      ref={dialog}
      className="source-dialog"
      aria-labelledby="source-heading"
      onCancel={(e) => {
        e.stopPropagation();
        e.preventDefault();
        back();
      }}
      onMouseDown={(e) => e.stopPropagation()}
    >
      <header className="source-header">
        <div className="source-crumb">
          <img alt="" src={brandIcon} />
          <strong>
            Meeting<b>Pal</b>
          </strong>
          <ChevronRight />
          {t("Meeting aufnehmen", "Record meeting")}
        </div>
        <div className="source-title">
          <h2 ref={heading} tabIndex={-1} id="source-heading">
            {page === "list"
              ? t("Aufnahmequelle wählen", "Choose a recording source")
              : page === "crop"
                ? t("Aufnahmebereich festlegen", "Set recording region")
                : t("Quellenvorschau", "Source preview")}
          </h2>
          {page === "list" && (
            <button
              type="button"
              className="source-icon-button"
              aria-label={t("Quellen aktualisieren", "Refresh sources")}
              title={t("Quellen aktualisieren", "Refresh sources")}
              disabled={loading || portalBusy}
              onClick={() => setRevision((r) => r + 1)}
            >
              <RefreshCw />
            </button>
          )}
          <button
            type="button"
            className="source-icon-button"
            aria-label={
              page === "list"
                ? t("Auswahl schließen", "Close picker")
                : t("Zurück", "Back")
            }
            onClick={back}
          >
            <X />
          </button>
        </div>
      </header>
      {page === "crop" &&
      selected &&
      frame &&
      (detail !== undefined || isGrant(selected.id)) ? (
        <CropEditor
          frame={frame}
          name={name}
          initial={crop}
          onApply={apply}
          t={t}
        />
      ) : (
        <>
          <div className="source-body">
            {page === "list" ? (
              <>
                {sources.some((s) => s.kind === "portal") && (
                  <button
                    type="button"
                    className="source-button portal-pick"
                    disabled={portalBusy}
                    onClick={pickPortal}
                  >
                    <Monitor />
                    {t(
                      "Bildschirm oder Fenster im System auswählen",
                      "Choose a screen or window in the system picker",
                    )}
                  </button>
                )}
                {error && (
                  <p role="alert" className="error-text">
                    {diagnostic(error, t)}
                  </p>
                )}
                <div
                  role="radiogroup"
                  aria-label={t("Aufnahmequelle", "Recording source")}
                >
                  {group("screen")}
                  {group("window")}
                </div>
              </>
            ) : (
              <div className="source-large">
                {page === "preview" && frame ? (
                  <Picture
                    frame={frame}
                    crop={crop}
                    label={t("Vorschau von ", "Preview of ") + name}
                  />
                ) : detail === undefined && !isGrant(selected?.id) ? (
                  waiting
                ) : (
                  unavailable
                )}
              </div>
            )}
          </div>
          <footer className="source-footer">
            <div className="source-summary" aria-live="polite">
              <CheckCircle2 />
              <div>
                <strong>
                  {name || t("Keine Quelle gewählt", "No source selected")}
                </strong>
                {selected && (
                  <small>
                    {cropFits
                      ? description(selected, crop, t)
                      : t(
                          "Auflösung geändert. Bitte Bereich neu wählen.",
                          "Resolution changed. Select the region again.",
                        )}
                  </small>
                )}
              </div>
            </div>
            <div className="source-actions">
              {page === "list" ? (
                <button
                  type="button"
                  className="source-button"
                  disabled={!selected}
                  onClick={() => openPage("preview")}
                >
                  <Maximize2 />
                  {t("Vorschau", "Preview")}
                </button>
              ) : (
                <button type="button" className="source-button" onClick={back}>
                  {t("Zurück", "Back")}
                </button>
              )}
              {page !== "crop" && editButton}
              {page !== "crop" && (
                <button
                  type="button"
                  className="source-button primary"
                  disabled={!selected || !cropFits}
                  onClick={() => apply(crop)}
                >
                  {t("Quelle übernehmen", "Use source")}
                </button>
              )}
            </div>
          </footer>
        </>
      )}
    </dialog>
  );
}
