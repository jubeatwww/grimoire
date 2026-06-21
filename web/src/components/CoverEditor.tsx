import { useEffect, useRef, useState } from "react";
import { editWork, uploadAsset } from "../api/client";

interface CoverEditorProps {
  itemId: string;
  current: string | null;
  /// Browse / Detail layouts use different placeholder sizing; the parent owns
  /// the "no game_work yet" empty state, so we always render an editor here.
  onChanged: () => void;
}

export function CoverEditor({ itemId, current, onChanged }: CoverEditorProps) {
  const [open, setOpen] = useState(false);
  const [urlDraft, setUrlDraft] = useState(current ?? "");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement | null>(null);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => setUrlDraft(current ?? ""), [current, itemId]);

  // Click outside to close.
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  // Ctrl/Cmd+V with an image on the clipboard → upload directly. Listens
  // globally while the popover is open so the user doesn't have to focus
  // anything specific (screenshot tools, "copy image" from a browser, etc.).
  useEffect(() => {
    if (!open) return;
    const handler = async (e: ClipboardEvent) => {
      const items = e.clipboardData?.items;
      if (!items) return;
      for (const it of Array.from(items)) {
        if (it.kind === "file" && it.type.startsWith("image/")) {
          const file = it.getAsFile();
          if (!file) continue;
          e.preventDefault();
          setBusy(true);
          setError(null);
          try {
            const url = await uploadAsset(file);
            await editWork(itemId, { coverImageUrl: url });
            onChanged();
            setOpen(false);
          } catch (err) {
            setError(err instanceof Error ? err.message : "Paste upload failed");
          } finally {
            setBusy(false);
          }
          return;
        }
      }
    };
    window.addEventListener("paste", handler);
    return () => window.removeEventListener("paste", handler);
  }, [open, itemId, onChanged]);

  const apply = async (next: string) => {
    setBusy(true);
    setError(null);
    try {
      await editWork(itemId, { coverImageUrl: next });
      onChanged();
      setOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Save failed");
    } finally {
      setBusy(false);
    }
  };

  const handleFile = async (file: File) => {
    setBusy(true);
    setError(null);
    try {
      const url = await uploadAsset(file);
      await editWork(itemId, { coverImageUrl: url });
      onChanged();
      setOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setBusy(false);
    }
  };

  return (
    <div ref={rootRef} className="cover-editor">
      {current ? (
        <img className="large-cover large-cover-image" src={current} alt="" />
      ) : (
        <div className="large-cover" />
      )}
      <button
        type="button"
        className="cover-edit-trigger"
        onClick={() => setOpen((v) => !v)}
        title="Change cover"
      >
        ✎
      </button>
      {open && (
        <div className="cover-edit-popover">
          <input
            ref={fileRef}
            type="file"
            accept="image/*"
            style={{ display: "none" }}
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) void handleFile(f);
              e.target.value = "";
            }}
          />
          <button
            type="button"
            className="cover-edit-btn primary"
            onClick={() => fileRef.current?.click()}
            disabled={busy}
          >
            {busy ? "…" : "Upload file"}
          </button>
          <div className="cover-edit-hint">
            …or paste an image with <kbd>Ctrl/⌘+V</kbd>
          </div>
          <div className="cover-edit-or">or</div>
          <input
            type="text"
            className="cover-edit-url"
            value={urlDraft}
            onChange={(e) => setUrlDraft(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && void apply(urlDraft.trim())}
            placeholder="Paste image URL"
          />
          <div className="cover-edit-actions">
            <button
              type="button"
              className="cover-edit-btn"
              onClick={() => void apply(urlDraft.trim())}
              disabled={busy}
            >
              Save URL
            </button>
            {current && (
              <button
                type="button"
                className="cover-edit-remove"
                onClick={() => void apply("")}
                disabled={busy}
              >
                Remove cover
              </button>
            )}
          </div>
          {error && <p className="cover-edit-error">{error}</p>}
        </div>
      )}
    </div>
  );
}
