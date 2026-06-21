import { PRIMARY_CATEGORIES } from "../App";
import { useEffect, useRef, useState } from "react";
import {
  createManualEntry,
  deleteInventoryItem,
  deleteItemAndFile,
  downloadUrl,
  editInventoryItem,
  editWork,
  excludeInventoryItem,
  refreshMetadata,
  resetInventoryItem,
} from "../api/client";
import type { InventoryItem } from "../api/types";
import { useConfirm } from "./ConfirmDialog";
import { CoverEditor } from "./CoverEditor";
import { InlineText } from "./InlineText";
import { SamplesEditor } from "./SamplesEditor";

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
  const confirm = useConfirm();
  const [menuOpen, setMenuOpen] = useState(false);
  const menuRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [menuOpen]);

  const handleRefresh = async (source: "dlsite" | "vndb" | "steam") => {
    try {
      await refreshMetadata(item.id, source);
      onChanged?.();
    } catch (e) {
      console.error("refresh failed", e);
    }
  };

  const handleReset = async () => {
    const ok = await confirm({
      title: "Reset this item?",
      message:
        "It will go back to pending. The metadata link is removed; the file is untouched.",
      confirmLabel: "Reset",
    });
    if (!ok) return;
    try {
      await resetInventoryItem(item.id);
      onChanged?.();
    } catch (e) {
      console.error("reset failed", e);
    }
  };

  const handleCreateManual = async () => {
    try {
      await createManualEntry(item.id);
      onChanged?.();
    } catch (e) {
      console.error("create manual entry failed", e);
    }
  };

  const handleExclude = async () => {
    const ok = await confirm({
      title: "Exclude this item?",
      message:
        "Marked as not-a-game (compilation / junk / utility) and hidden from Organize. Reset brings it back.",
      confirmLabel: "Exclude",
    });
    if (!ok) return;
    try {
      await excludeInventoryItem(item.id);
      onChanged?.();
    } catch (e) {
      console.error("exclude failed", e);
    }
  };

  const handleDeleteRecord = async () => {
    setMenuOpen(false);
    const ok = await confirm({
      title: "Delete this inventory record?",
      message: (
        <>
          The file itself is left untouched on disk (or already gone). The linked
          game_work is kept so other items that share it stay intact.
        </>
      ),
      confirmLabel: "Delete record",
      danger: true,
    });
    if (!ok) return;
    try {
      await deleteInventoryItem(item.id);
      onChanged?.();
    } catch (e) {
      console.error("delete failed", e);
    }
  };

  const handleDeleteWithFile = async () => {
    setMenuOpen(false);
    const ok = await confirm({
      title: "Delete file and record?",
      message: (
        <>
          <p style={{ margin: "0 0 6px" }}>
            <strong>{item.fileName}</strong>
          </p>
          <p style={{ margin: 0 }}>
            The file under the library root will be <em>permanently removed
            from disk</em>, then this inventory record is dropped. The linked
            game_work is kept. Use this to clean up duplicates — it can't be
            undone from the app.
          </p>
        </>
      ),
      confirmLabel: "Delete file + record",
      danger: true,
    });
    if (!ok) return;
    try {
      await deleteItemAndFile(item.id);
      onChanged?.();
    } catch (e) {
      console.error("delete file failed", e);
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

  const hasGameWork = item.displayTitle != null;
  const isManualEntry =
    item.organizationStatus === "confirmed" &&
    !item.dlsiteWorkId &&
    !item.vndbId &&
    !item.steamAppId;

  return (
    <div className="item-focus">
      {item.missing && (
        <div className="item-missing-banner">
          ⚠ File missing — the scanner couldn't find this path anymore.
        </div>
      )}
      {!hideMedia &&
        (hasGameWork ? (
          <CoverEditor
            itemId={item.id}
            current={item.coverImageUrl}
            onChanged={() => onChanged?.()}
          />
        ) : item.coverImageUrl ? (
          <img className="large-cover large-cover-image" src={item.coverImageUrl} alt="" />
        ) : (
          <div className="large-cover" />
        ))}
      {!hasGameWork && (
        <div className="item-focus-manual">
          <p>No metadata linked yet.</p>
          <button
            type="button"
            className="item-focus-manual-btn"
            onClick={handleCreateManual}
          >
            + Create manual entry
          </button>
          <p className="item-focus-manual-hint">
            Creates a blank record using the filename as the title; you can then
            edit everything inline.
          </p>
        </div>
      )}
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
        {isManualEntry && (
          <span className="manual-pill" title="No source linked — entered manually">
            manual
          </span>
        )}
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
      {!hideMedia && hasGameWork && (
        <SamplesEditor
          itemId={item.id}
          current={item.previewImageUrls ?? []}
          onChanged={() => onChanged?.()}
        />
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
          <div ref={menuRef} className="action-menu">
            <button
              type="button"
              className="action-menu-trigger"
              onClick={() => setMenuOpen((v) => !v)}
              title="More actions"
              aria-haspopup="menu"
              aria-expanded={menuOpen}
            >
              ⋯
            </button>
            {menuOpen && (
              <div className="action-menu-popover" role="menu">
                <button
                  type="button"
                  className="action-menu-item"
                  onClick={handleDeleteRecord}
                >
                  <span>🗑 Delete record only</span>
                  <small>keep file on disk</small>
                </button>
                <button
                  type="button"
                  className="action-menu-item action-menu-danger"
                  onClick={handleDeleteWithFile}
                >
                  <span>💣 Delete file + record</span>
                  <small>removes the file from {item.legacyLocation ?? "library"}/…</small>
                </button>
              </div>
            )}
          </div>
        </div>
    </div>
  );
}
