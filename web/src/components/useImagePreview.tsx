import { useState, type CSSProperties, type MouseEvent, type ReactNode } from "react";

const PREVIEW_W = 480;
const PREVIEW_H = 360;
const PAD = 12;

interface PreviewState {
  src: string;
  rect: DOMRect;
}

function position(rect: DOMRect): CSSProperties {
  const w = Math.min(PREVIEW_W, window.innerWidth - 2 * PAD);
  const h = Math.min(PREVIEW_H, window.innerHeight - 2 * PAD);
  const placeLeft = window.innerWidth - rect.right < w + PAD;
  const left = placeLeft
    ? Math.max(PAD, rect.left - w - PAD)
    : Math.min(rect.right + PAD, window.innerWidth - w - PAD);
  const idealTop = rect.top + rect.height / 2 - h / 2;
  const top = Math.max(PAD, Math.min(idealTop, window.innerHeight - h - PAD));
  return { top, left, width: w, height: h };
}

export function useImagePreview(): {
  hoverProps: (src: string | null | undefined) => Record<string, unknown>;
  preview: ReactNode;
  clear: () => void;
} {
  const [state, setState] = useState<PreviewState | null>(null);
  const clear = () => setState(null);

  const hoverProps = (src: string | null | undefined) =>
    src
      ? {
          onMouseEnter: (e: MouseEvent<HTMLElement>) =>
            setState({ src, rect: e.currentTarget.getBoundingClientRect() }),
          onMouseLeave: clear,
        }
      : {};

  const preview = state ? (
    <div className="hover-preview" style={position(state.rect)}>
      <img src={state.src} alt="" />
    </div>
  ) : null;

  return { hoverProps, preview, clear };
}
