import { useEffect, useRef, useState } from "react";
import type { InventoryItem, OrganizationStatus } from "../api/types";
import { ItemFocus } from "./ItemFocus";
import { SearchPanel } from "./SearchPanel";

interface OrganizeModeProps {
  items: InventoryItem[];
  autoSearchToken: number;
  onAutoTrigger: () => void;
  onSkip: (item: InventoryItem) => void;
  onMetadataConfirmed: () => void;
  onExit: () => void;
}

type Filter = "pending" | "needs-detail" | "no-dlsite";
const ALL_FILTERS: { id: Filter; label: string; desc: string }[] = [
  { id: "pending", label: "Pending", desc: "no source match yet" },
  { id: "needs-detail", label: "Confirmed · missing detail", desc: "never enriched" },
  { id: "no-dlsite", label: "No DLsite match", desc: "skipped before — try VNDB" },
];

function inQueue(item: InventoryItem, filters: Set<Filter>): boolean {
  if (filters.has("pending") && item.organizationStatus === "pending") return true;
  if (
    filters.has("needs-detail") &&
    item.organizationStatus === "confirmed" &&
    !item.enrichedAt
  ) {
    return true;
  }
  if (filters.has("no-dlsite") && item.organizationStatus === "no_match") return true;
  return false;
}

const STATUS_ICON: Record<OrganizationStatus, string> = {
  pending: "○",
  matched: "◐",
  confirmed: "✓",
  ignored: "—",
  no_match: "✗",
};

export function OrganizeMode({
  items,
  autoSearchToken,
  onAutoTrigger,
  onSkip,
  onMetadataConfirmed,
  onExit,
}: OrganizeModeProps) {
  const [filters, setFilters] = useState<Set<Filter>>(
    () => new Set<Filter>(["pending", "needs-detail"]),
  );
  const [snapshotIds, setSnapshotIds] = useState<string[]>([]);
  const [pos, setPos] = useState(0);
  const built = useRef(false);

  const rebuild = () => {
    const ids = items.filter((i) => inQueue(i, filters)).map((i) => i.id);
    setSnapshotIds(ids);
    setPos(0);
    built.current = true;
  };

  useEffect(() => {
    if (built.current) rebuild();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters]);

  useEffect(() => {
    if (!built.current && items.length > 0) rebuild();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items.length]);

  const currentId = snapshotIds[pos] ?? null;
  const current = currentId ? items.find((i) => i.id === currentId) ?? null : null;

  useEffect(() => {
    if (current) onAutoTrigger();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentId]);

  const goPrev = () => {
    if (pos > 0) setPos(pos - 1);
  };
  const goNext = () => {
    if (pos < snapshotIds.length - 1) setPos(pos + 1);
  };
  const skipCurrent = () => {
    if (!current) return;
    onSkip(current);
    goNext();
  };

  // Keyboard nav. Skip when focus is in an editable field so the user can type
  // RJ codes / titles without the queue scrolling under them.
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
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (e.key === "ArrowLeft") {
        e.preventDefault();
        goPrev();
      } else if (e.key === "ArrowRight") {
        e.preventDefault();
        goNext();
      } else if (e.key === "s" || e.key === "S") {
        e.preventDefault();
        skipCurrent();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pos, snapshotIds.length, currentId]);

  const toggleFilter = (f: Filter) => {
    setFilters((prev) => {
      const next = new Set(prev);
      next.has(f) ? next.delete(f) : next.add(f);
      return next;
    });
  };

  return (
    <div className="organize-3col">
      <header className="organize-header">
        <button className="organize-exit" onClick={onExit} title="Back to library">
          ←
        </button>
        <div className="organize-progress">
          {snapshotIds.length > 0 ? (
            <>
              <strong>
                {pos + 1} / {snapshotIds.length}
              </strong>
              <span className="organize-progress-label">to organize</span>
            </>
          ) : (
            <strong className="organize-progress-label">Queue empty</strong>
          )}
        </div>
        <div className="organize-filters">
          {ALL_FILTERS.map((f) => (
            <button
              key={f.id}
              type="button"
              className={`chip ${filters.has(f.id) ? "chip-on" : ""}`}
              onClick={() => toggleFilter(f.id)}
              title={f.desc}
            >
              {f.label}
            </button>
          ))}
          <button
            type="button"
            className="organize-rebuild"
            onClick={rebuild}
            title="Rebuild queue from current filters (drops processed)"
          >
            ↻ Rebuild
          </button>
        </div>
        <div className="organize-actions">
          <button onClick={goPrev} disabled={pos === 0} title="← Prev">
            ← Prev
          </button>
          <button
            onClick={skipCurrent}
            disabled={!current}
            className="organize-skip"
            title="S · Skip · mark as no DLsite match"
          >
            Skip (S)
          </button>
          <button
            onClick={goNext}
            disabled={pos >= snapshotIds.length - 1}
            className="organize-next"
            title="→ Next"
          >
            Next →
          </button>
        </div>
      </header>

      <div className="organize-body">
        <aside className="organize-sidebar">
          {snapshotIds.length === 0 ? (
            <p className="organize-sidebar-empty">Queue empty.</p>
          ) : (
            <ul className="organize-queue">
              {snapshotIds.map((id, i) => {
                const it = items.find((x) => x.id === id);
                if (!it) return null;
                const active = i === pos;
                const processed = it.organizationStatus !== "pending";
                const cls = [
                  "organize-queue-item",
                  active ? "active" : "",
                  processed ? "processed" : "",
                ]
                  .filter(Boolean)
                  .join(" ");
                return (
                  <li key={id}>
                    <button
                      type="button"
                      className={cls}
                      onClick={() => setPos(i)}
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

        <main className="organize-middle">
          {current ? (
            <ItemFocus item={current} onChanged={onMetadataConfirmed} />
          ) : (
            <div className="organize-empty">
              <h2>Nothing in the queue</h2>
              <p>Toggle filters above or run a scan to populate.</p>
            </div>
          )}
        </main>

        <aside className="organize-right">
          {current && (
            <SearchPanel
              item={current}
              autoSearchToken={autoSearchToken}
              onChanged={onMetadataConfirmed}
            />
          )}
        </aside>
      </div>
    </div>
  );
}
