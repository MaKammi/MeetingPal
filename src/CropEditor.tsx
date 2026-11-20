import { useRef, useState, type PointerEvent } from "react";
import { Monitor, MousePointer2, Scan } from "lucide-react";
import type { Crop, SourcePreview, Translate } from "./types";

const minimum = 16;
const clamp = (n: number, low: number, high: number) =>
  Math.max(low, Math.min(n, high));
type Point = { x: number; y: number };
const corners = ["nw", "ne", "sw", "se"] as const;
type Drag = {
  pointer: number;
  mode: "draw" | "move" | (typeof corners)[number];
  start: Point;
  original: Crop | null;
};

export const dimensions = (size: { width: number; height: number }) =>
  size.width + " × " + size.height + " px";

export function fits(crop: Crop, frame: { width: number; height: number }) {
  return (
    crop.x >= 0 &&
    crop.y >= 0 &&
    crop.width >= minimum &&
    crop.height >= minimum &&
    crop.x + crop.width <= frame.width &&
    crop.y + crop.height <= frame.height
  );
}

function dragged(drag: Drag, point: Point, frame: SourcePreview): Crop {
  const { start, original, mode } = drag;
  if (mode === "draw" || !original) {
    return {
      x: Math.min(start.x, point.x),
      y: Math.min(start.y, point.y),
      width: Math.abs(start.x - point.x),
      height: Math.abs(start.y - point.y),
    };
  }
  if (mode === "move") {
    return {
      ...original,
      x: clamp(original.x + point.x - start.x, 0, frame.width - original.width),
      y: clamp(
        original.y + point.y - start.y,
        0,
        frame.height - original.height,
      ),
    };
  }
  // Retain the opposite corner and the offset between the pointer and the handle.
  const right = original.x + original.width;
  const bottom = original.y + original.height;
  const dx = point.x - start.x,
    dy = point.y - start.y;
  const west = mode.endsWith("w"),
    north = mode.startsWith("n");
  const x = west ? clamp(original.x + dx, 0, right - minimum) : original.x;
  const y = north ? clamp(original.y + dy, 0, bottom - minimum) : original.y;
  return {
    x,
    y,
    width: (west ? right : clamp(right + dx, x + minimum, frame.width)) - x,
    height:
      (north ? bottom : clamp(bottom + dy, y + minimum, frame.height)) - y,
  };
}

export function CropEditor({
  frame,
  name,
  initial,
  onApply,
  t,
}: {
  frame: SourcePreview;
  name: string;
  initial: Crop | null;
  onApply: (crop: Crop | null) => void;
  t: Translate;
}) {
  const [crop, setCrop] = useState(
    initial && fits(initial, frame) ? initial : null,
  );
  const gesture = useRef<Drag | null>(null);
  const stage = useRef<HTMLButtonElement>(null);
  const point = (event: PointerEvent<HTMLButtonElement>): Point => {
    const box = event.currentTarget.getBoundingClientRect();
    return {
      x: clamp(
        Math.round(((event.clientX - box.left) * frame.width) / box.width),
        0,
        frame.width,
      ),
      y: clamp(
        Math.round(((event.clientY - box.top) * frame.height) / box.height),
        0,
        frame.height,
      ),
    };
  };
  function cancelDrag() {
    if (!gesture.current) return;
    setCrop(gesture.current.original);
    gesture.current = null;
  }
  return (
    <>
      <div className="source-body">
        <div className="source-crop-title">
          <h3>
            <Monitor />
            {name}
          </h3>
          <small>{dimensions(frame)}</small>
        </div>
        <div className="crop-mat">
          <button
            ref={stage}
            type="button"
            className="crop-stage"
            style={{
              aspectRatio: frame.width + " / " + frame.height,
              width: "min(100%, " + (48 * frame.width) / frame.height + "vh)",
            }}
            aria-label={t(
              "Aufnahmebereich in der Bildschirmvorschau auswählen",
              "Select a region in the screen preview",
            )}
            aria-describedby="crop-help crop-keys"
            onPointerDown={(e) => {
              if (gesture.current || e.button !== 0 || !e.isPrimary) return;
              e.preventDefault();
              e.currentTarget.focus({ preventScroll: true });
              const target = e.target as HTMLElement;
              const mode = target.closest<HTMLElement>("[data-drag]")?.dataset
                .drag as Drag["mode"] | undefined;
              const drag: Drag = {
                mode: mode ?? "draw",
                pointer: e.pointerId,
                start: point(e),
                original: crop,
              };
              gesture.current = drag;
              e.currentTarget.setPointerCapture(e.pointerId);
              if (drag.mode === "draw")
                setCrop(dragged(drag, drag.start, frame));
            }}
            onPointerMove={(e) => {
              const drag = gesture.current;
              if (drag?.pointer === e.pointerId)
                setCrop(dragged(drag, point(e), frame));
            }}
            onPointerUp={(e) => {
              const drag = gesture.current;
              if (drag?.pointer !== e.pointerId) return;
              const next = dragged(drag, point(e), frame);
              gesture.current = null;
              setCrop(fits(next, frame) ? next : drag.original);
              e.currentTarget.releasePointerCapture(e.pointerId);
            }}
            onLostPointerCapture={(e) => {
              if (gesture.current?.pointer === e.pointerId) cancelDrag();
            }}
            onPointerCancel={cancelDrag}
            onKeyDown={(e) => {
              if (e.key === "Escape" && gesture.current) {
                e.preventDefault();
                const pointer = gesture.current.pointer;
                cancelDrag();
                e.currentTarget.releasePointerCapture(pointer);
              } else if (e.key === "Enter" && !crop) {
                e.preventDefault();
                const width = Math.max(minimum, Math.round(frame.width / 2));
                const height = Math.max(minimum, Math.round(frame.height / 2));
                setCrop({
                  x: Math.floor((frame.width - width) / 2),
                  y: Math.floor((frame.height - height) / 2),
                  width,
                  height,
                });
              } else if (
                crop &&
                ["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(
                  e.key,
                )
              ) {
                e.preventDefault();
                const dx =
                  Number(e.key === "ArrowRight") -
                  Number(e.key === "ArrowLeft");
                const dy =
                  Number(e.key === "ArrowDown") - Number(e.key === "ArrowUp");
                setCrop(
                  e.shiftKey
                    ? {
                        ...crop,
                        width: clamp(
                          crop.width + dx,
                          minimum,
                          frame.width - crop.x,
                        ),
                        height: clamp(
                          crop.height + dy,
                          minimum,
                          frame.height - crop.y,
                        ),
                      }
                    : {
                        ...crop,
                        x: clamp(crop.x + dx, 0, frame.width - crop.width),
                        y: clamp(crop.y + dy, 0, frame.height - crop.height),
                      },
                );
              }
            }}
          >
            <img src={frame.image} alt="" draggable={false} />
            {crop && (
              <span
                className="crop-selection"
                data-drag="move"
                style={{
                  left: (100 * crop.x) / frame.width + "%",
                  top: (100 * crop.y) / frame.height + "%",
                  width: (100 * crop.width) / frame.width + "%",
                  height: (100 * crop.height) / frame.height + "%",
                }}
              >
                {corners.map((corner) => (
                  <span
                    key={corner}
                    className={"crop-corner " + corner}
                    data-drag={corner}
                  />
                ))}
              </span>
            )}
          </button>
        </div>
        <div className="crop-hint">
          <p id="crop-help">
            <MousePointer2 />
            {crop
              ? t(
                  "Verschiebe den Rahmen oder ziehe an seinen Ecken.",
                  "Move the frame or drag its corners.",
                )
              : t(
                  "Ziehe mit der Maus ein Rechteck auf.",
                  "Drag to draw a rectangle.",
                )}
          </p>
          <button
            type="button"
            style={{ visibility: crop ? "visible" : "hidden" }}
            onClick={() => {
              setCrop(null);
              stage.current?.focus();
            }}
          >
            {t("Neu aufziehen", "Draw again")}
          </button>
        </div>
        <span className="source-sr" id="crop-keys">
          {t(
            "Eingabe wählt einen mittigen Bereich. Mit den Pfeiltasten verschieben, mit Umschalt und Pfeiltasten die Größe ändern.",
            "Press Enter to select a centered area. Use the arrow keys to move it or Shift and the arrow keys to resize it.",
          )}
        </span>
      </div>
      <footer className="source-footer">
        <div className="source-summary">
          <Scan />
          <div>
            <strong>{t("Aufnahmebereich", "Recording region")}</strong>
            <small aria-live="polite">
              {crop
                ? dimensions(crop)
                : t("Noch kein Bereich gewählt", "No region selected")}
            </small>
          </div>
        </div>
        <div className="source-actions">
          <button
            type="button"
            className="source-button"
            onClick={() => onApply(null)}
          >
            {t("Ganzer Bildschirm", "Entire screen")}
          </button>
          <button
            type="button"
            className="source-button primary"
            disabled={!crop || !fits(crop, frame)}
            onClick={() => onApply(crop)}
          >
            {t("Bereich übernehmen", "Use region")}
          </button>
        </div>
      </footer>
    </>
  );
}
