import { useEffect, useState } from "react";
import type { InventoryItem, OrganizationStatus } from "../api/types";
import { Gallery } from "./Gallery";
import { ItemFocus } from "./ItemFocus";
import { SearchPanel } from "./SearchPanel";

interface BrowseModeProps {
  items: InventoryItem[];
  selectedId: string | null;
  autoSearchToken: number;
  onSelect: (item: InventoryItem) => void;
  onMetadataConfirmed: () => void;
}

const STATUS_ICON: Record<OrganizationStatus, string> = {
  pending: "○",
  matched: "◐",
  confirmed: "✓",
  ignored: "—",
  no_match: "✗",
};

export function BrowseMode({
  items,
  selectedId,
  autoSearchToken,
  onSelect,
  onMetadataConfirmed,
}: BrowseModeProps) {
  const [matchExpanded, setMatchExpanded] = useState(false);

  const currentIdx = Math.max(0, items.findIndex((i) => i.id === selectedId));
  const current = items[currentIdx] ?? items[0] ?? null;

  // Auto-pick the first item when the list loads or selection falls out of it.
  useEffect(() => {
    if (items.length > 0 && !items.find((i) => i.id === selectedId)) {
      onSelect(items[0]);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items.length === 0 ? null : items[0]?.id, selectedId]);

  // Item nav: ↑ / ↓. Skip when typing.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (
        t &&
        (t.tagName === "INPUT" ||
          t.tagName === "TEXTAREA" ||
          t.tagName === "SELECT" ||
          t.isContentEditable)
      ) {
        return;
      }
      if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
      if (e.key === "ArrowUp") {
        e.preventDefault();
        const prev = items[currentIdx - 1];
        if (prev) onSelect(prev);
      } else if (e.key === "ArrowDown") {
        e.preventDefault();
        const next = items[currentIdx + 1];
        if (next) onSelect(next);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [currentIdx, items, onSelect]);

  // Collapse the match panel when switching items so it doesn't follow you.
  useEffect(() => {
    setMatchExpanded(false);
  }, [current?.id]);

  return (
    <div className="browse-mode">
      <aside className="browse-sidebar">
        {items.length === 0 ? (
          <p className="browse-sidebar-empty">No items match the current filters.</p>
        ) : (
          <ul className="browse-queue">
            {items.map((it, i) => {
              const active = i === currentIdx;
              const cls = ["browse-queue-item", active ? "active" : ""]
                .filter(Boolean)
                .join(" ");
              return (
                <li key={it.id}>
                  <button
                    type="button"
                    className={cls}
                    onClick={() => onSelect(it)}
                    title={it.fileName}
                  >
                    <span className={`queue-status status-${it.organizationStatus}`}>
                      {STATUS_ICON[it.organizationStatus]}
                    </span>
                    <span className="queue-thumb">
                      {it.coverImageUrl ? (
                        <img src={it.coverImageUrl} alt="" loading="lazy" />
                      ) : (
                        <span className="queue-thumb-fallback" />
                      )}
                    </span>
                    <span className="queue-title">
                      {it.displayTitle ?? it.fileName}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </aside>

      <main className="browse-main">
        {current ? (
          <>
            <Gallery
              itemKey={current.id}
              cover={current.coverImageUrl}
              samples={current.previewImageUrls ?? []}
            />
            <ItemFocus item={current} onChanged={onMetadataConfirmed} hideMedia />
            <section className="browse-match">
              <button
                type="button"
                className="browse-match-toggle"
                onClick={() => setMatchExpanded((x) => !x)}
                aria-expanded={matchExpanded}
              >
                {matchExpanded ? "▾" : "▸"} Match metadata
              </button>
              {matchExpanded && (
                <SearchPanel
                  item={current}
                  autoSearchToken={autoSearchToken}
                  onChanged={onMetadataConfirmed}
                />
              )}
            </section>
          </>
        ) : (
          <div className="browse-empty">
            <h2>Nothing to browse</h2>
            <p>Run a scan or relax your filters.</p>
          </div>
        )}
      </main>
    </div>
  );
}
