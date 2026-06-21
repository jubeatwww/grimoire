import { useEffect } from "react";
import { PRIMARY_CATEGORIES } from "../App";
import {
  downloadUrl,
  editInventoryItem,
  editWork,
  excludeInventoryItem,
  refreshMetadata,
  resetInventoryItem,
} from "../api/client";
import type { InventoryItem } from "../api/types";
import { InlineText } from "./InlineText";
import { useImagePreview } from "./useImagePreview";

interface ItemFocusProps {
  item: InventoryItem;
  onChanged?: () => void;
  /// Browse mode renders its own Gallery for cover + samples, so ItemFocus
  /// should skip its own preview-image grid and (optionally) the large cover.
  hideMedia?: boolean;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 2 : 1)} ${units[i]}`;
}

export function ItemFocus({ item, onChanged, hideMedia }: ItemFocusProps) {
  const { hoverProps, preview, clear: clearPreview } = useImagePreview();

  // Drop any pending hover preview when the focused item changes — the anchor
  // image unmounts so mouseleave never fires on its own.
  useEffect(() => clearPreview(), [item.id, clearPreview]);

  const handleRefresh = async (source: "dlsite" | "vndb" | "steam") => {
    try {
      await refreshMetadata(item.id, source);
      onChanged?.();
    } catch (e) {
      console.error("refresh failed", e);
    }
  };

  const handleReset = async () => {
    if (!confirm("Reset this item back to pending? Existing metadata link will be removed.")) return;
    try {
      await resetInventoryItem(item.id);
      onChanged?.();
    } catch (e) {
      console.error("reset failed", e);
    }
  };

  const handleExclude = async () => {
    if (
      !confirm(
        "Exclude this item? It will be marked as not-a-game and stay out of the Organize queue. Reset to bring it back.",
      )
    ) {
      return;
    }
    try {
      await excludeInventoryItem(item.id);
      onChanged?.();
    } catch (e) {
      console.error("exclude failed", e);
    }
  };

  const saveTitle = async (next: string) => {
    await editWork(item.id, { displayTitle: next });
    onChanged?.();
  };

  const saveWorkTypeLabel = async (next: string) => {
    await editWork(item.id, {
      workTypeLabel: next,
      workType: next.trim() === "" ? "" : undefined,
    });
    onChanged?.();
  };

  const savePrimaryCategory = async (next: string) => {
    await editInventoryItem(item.id, { primaryCategory: next });
    onChanged?.();
  };

  return (
    <div className="item-focus">
      {!hideMedia && (item.coverImageUrl ? (
        <img className="large-cover large-cover-image" src={item.coverImageUrl} alt="" />
      ) : (
        <div className="large-cover" />
      ))}
      <div className="detail-title-row">
        <h2>
          {item.displayTitle ? (
            <InlineText
              value={item.displayTitle}
              onSave={saveTitle}
              placeholder="Set title"
            />
          ) : (
            item.fileName
          )}
        </h2>
        <a
          className="icon-button"
          href={downloadUrl(item.id)}
          title="Download"
          aria-label="Download"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M12 3v12" />
            <path d="m6 11 6 6 6-6" />
            <path d="M5 21h14" />
          </svg>
        </a>
      </div>
      {item.displayTitle && <p className="detail-filename">{item.fileName}</p>}
      {item.circle && <p className="detail-circle">{item.circle}</p>}
      <div className="detail-status-row">
        <span className={`status-pill status-${item.organizationStatus}`}>
          {item.organizationStatus}
        </span>
        {item.displayTitle ? (
          <span className="work-type-pill" title={item.workType ?? undefined}>
            <InlineText
              value={item.workTypeLabel ?? item.workType ?? ""}
              onSave={saveWorkTypeLabel}
              placeholder="add type"
            />
          </span>
        ) : (
          (item.workTypeLabel || item.workType) && (
            <span className="work-type-pill" title={item.workType ?? undefined}>
              {item.workTypeLabel ?? item.workType}
            </span>
          )
        )}
        <select
          className="primary-select"
          value={item.primaryCategory ?? ""}
          onChange={(e) => void savePrimaryCategory(e.target.value)}
          title="Primary category"
        >
          <option value="">Unsorted</option>
          {PRIMARY_CATEGORIES.map((c) => (
            <option key={c} value={c}>{c}</option>
          ))}
        </select>
        {item.fileType && <span>· {item.fileType}</span>}
        {item.fileSizeBytes != null && (
          <span>· {formatBytes(item.fileSizeBytes)}</span>
        )}
      </div>
      {item.rateAverage != null && item.rateCount != null && item.rateCount > 0 && (
        <p className="detail-rating">
          <span className="rating-stars">{"★".repeat(Math.round(item.rateAverage))}</span>
          <span>{item.rateAverage.toFixed(2)}</span>
          <span className="rating-count">({item.rateCount.toLocaleString()})</span>
          {item.dlCount != null && (
            <span className="rating-dlcount">· {item.dlCount.toLocaleString()} DL</span>
          )}
        </p>
      )}
      {item.sourceTags && item.sourceTags.length > 0 && (
        <div className="detail-tags">
          {item.sourceTags.map((t) => (
            <span key={t} className="detail-tag">{t}</span>
          ))}
        </div>
      )}
      {item.description && (
        <p className="detail-description">{item.description}</p>
      )}
      {!hideMedia && item.previewImageUrls && item.previewImageUrls.length > 0 && (
        <div className="detail-previews">
          {item.previewImageUrls.map((url) => (
            <a
              key={url}
              href={url}
              target="_blank"
              rel="noreferrer"
              className="detail-preview"
              {...hoverProps(url)}
            >
              <img src={url} alt="" loading="lazy" />
            </a>
          ))}
        </div>
      )}
      <dl>
        {item.releaseDate && (
          <>
            <dt>Release</dt>
            <dd>{item.releaseDate}</dd>
          </>
        )}
        {item.series && (
          <>
            <dt>Series</dt>
            <dd>{item.series}</dd>
          </>
        )}
        <dt>Legacy location</dt>
        <dd>{item.legacyLocation ?? "none"}</dd>
        <dt>Version</dt>
        <dd>{item.version ?? "unknown"}</dd>
        <dt>Language</dt>
        <dd>{item.language ?? "unknown"}</dd>
      </dl>
      <div className="detail-source-actions">
          {item.dlsiteWorkId && (
            <button
              type="button"
              className="source-refresh source-refresh-dlsite"
              onClick={() => handleRefresh("dlsite")}
              title={`Refresh from DLsite (${item.dlsiteWorkId})`}
            >
              ↻ DLsite
            </button>
          )}
          {item.vndbId && (
            <button
              type="button"
              className="source-refresh source-refresh-vndb"
              onClick={() => handleRefresh("vndb")}
              title={`Refresh from VNDB (${item.vndbId})`}
            >
              ↻ VNDB
            </button>
          )}
          {item.steamAppId && (
            <button
              type="button"
              className="source-refresh source-refresh-steam"
              onClick={() => handleRefresh("steam")}
              title={`Refresh from Steam (${item.steamAppId})`}
            >
              ↻ Steam
            </button>
          )}
          {item.organizationStatus !== "pending" && (
            <button
              type="button"
              className="source-reset"
              onClick={handleReset}
              title="Unlink metadata and reset back to pending"
            >
              ↺ Reset
            </button>
          )}
          {item.organizationStatus !== "ignored" && (
            <button
              type="button"
              className="source-exclude"
              onClick={handleExclude}
              title="Mark as not-a-game (compilation / junk / utility) — hidden from Organize"
            >
              ✕ Exclude
            </button>
          )}
        </div>
      {preview}
    </div>
  );
}
