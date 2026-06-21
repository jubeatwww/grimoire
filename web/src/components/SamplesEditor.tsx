import { useEffect, useRef, useState } from "react";
import { editWork, uploadAsset } from "../api/client";
import { useImagePreview } from "./useImagePreview";

interface SamplesEditorProps {
  itemId: string;
  current: string[];
  onChanged: () => void;
}

export function SamplesEditor({ itemId, current, onChanged }: SamplesEditorProps) {
  const [open, setOpen] = useState(false);
  const [urlDraft, setUrlDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const fileRef = useRef<HTMLInputElement | null>(null);
  const editorRef = useRef<HTMLDivElement | null>(null);
  const { hoverProps, preview } = useImagePreview();

  // Click outside the editor closes the popover, but keep the hover preview
  // working — the preview overlay has pointer-events: none so it never
  // counts as "inside".
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (editorRef.current && !editorRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  // Paste image while popover is open.
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
          await uploadAndAppend(file);
          return;
        }
      }
    };
    window.addEventListener("paste", handler);
    return () => window.removeEventListener("paste", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, current.join("\n")]);

  const append = async (url: string) => {
    const trimmed = url.trim();
    if (!trimmed) return;
    if (current.includes(trimmed)) {
      setError("Already in the list");
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await editWork(itemId, { previewImageUrls: [...current, trimmed] });
      onChanged();
      setUrlDraft("");
      setOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Add failed");
    } finally {
      setBusy(false);
    }
  };

  const uploadAndAppend = async (file: File) => {
    setBusy(true);
    setError(null);
    try {
      const url = await uploadAsset(file);
      await editWork(itemId, { previewImageUrls: [...current, url] });
      onChanged();
      setOpen(false);
    } catch (e) {
      setError(e instanceof Error ? e.message : "Upload failed");
    } finally {
      setBusy(false);
    }
  };

  const remove = async (url: string) => {
    try {
      await editWork(itemId, {
        previewImageUrls: current.filter((u) => u !== url),
      });
      onChanged();
    } catch (e) {
      console.error("remove sample failed", e);
    }
  };

  return (
    <div ref={editorRef} className="samples-editor">
      <div className="detail-previews">
        {current.map((url) => (
          <div key={url} className="sample-tile" {...hoverProps(url)}>
            <a
              href={url}
              target="_blank"
              rel="noreferrer"
              className="detail-preview"
            >
              <img src={url} alt="" loading="lazy" />
            </a>
            <button
              type="button"
              className="sample-remove"
              onClick={() => void remove(url)}
              title="Remove image"
            >
              ✕
            </button>
          </div>
        ))}
        <button
          type="button"
          className="sample-add"
          onClick={() => setOpen((v) => !v)}
          title="Add sample image"
        >
          +
        </button>
      </div>
      {open && (
        <div className="cover-edit-popover samples-edit-popover">
          <input
            ref={fileRef}
            type="file"
            accept="image/*"
            multiple
            style={{ display: "none" }}
            onChange={async (e) => {
              const files = Array.from(e.target.files ?? []);
              e.target.value = "";
              for (const f of files) {
                await uploadAndAppend(f);
              }
            }}
          />
          <button
            type="button"
            className="cover-edit-btn primary"
            onClick={() => fileRef.current?.click()}
            disabled={busy}
          >
            {busy ? "…" : "Upload image(s)"}
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
            onKeyDown={(e) => e.key === "Enter" && void append(urlDraft)}
            placeholder="Paste image URL"
          />
          <div className="cover-edit-actions">
            <button
              type="button"
              className="cover-edit-btn"
              onClick={() => void append(urlDraft)}
              disabled={busy}
            >
              Add URL
            </button>
          </div>
          {error && <p className="cover-edit-error">{error}</p>}
        </div>
      )}
      {preview}
    </div>
  );
}
