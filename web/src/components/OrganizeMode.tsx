import { useEffect, useRef, useState } from "react";
import type { InventoryItem } from "../api/types";
import { DetailPanel } from "./DetailPanel";

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
  { id: "needs-detail", label: "Confirmed · missing detail", desc: "needs metadata refresh" },
  { id: "no-dlsite", label: "No DLsite match", desc: "skipped before — try VNDB" },
];

function inQueue(item: InventoryItem, filters: Set<Filter>): boolean {
  if (filters.has("pending") && item.organizationStatus === "pending") return true;
  if (
    filters.has("needs-detail") &&
    item.organizationStatus === "confirmed" &&
    !item.description
  ) {
    return true;
  }
  if (filters.has("no-dlsite") && item.organizationStatus === "no_match") return true;
  return false;
}

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
  // Snapshot keeps the queue stable so confirmed/skipped items stay visible for review.
  // It rebuilds on filter change or via the Rebuild button.
  const [snapshotIds, setSnapshotIds] = useState<string[]>([]);
  const [pos, setPos] = useState(0);
  const built = useRef(false);

  const rebuild = () => {
    const ids = items.filter((i) => inQueue(i, filters)).map((i) => i.id);
    setSnapshotIds(ids);
    setPos(0);
    built.current = true;
  };

  // Rebuild on filter changes.
  useEffect(() => {
    if (built.current) rebuild();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filters]);

  // First-load build once items arrive.
  useEffect(() => {
    if (!built.current && items.length > 0) rebuild();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [items.length]);

  const currentId = snapshotIds[pos] ?? null;
  const current = currentId ? items.find((i) => i.id === currentId) ?? null : null;

  // Auto-fire search/refresh in the embedded DetailPanel whenever we land on a new item.
  useEffect(() => {
    if (current) onAutoTrigger();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentId]);

  const toggleFilter = (f: Filter) => {
    setFilters((prev) => {
      const next = new Set(prev);
      next.has(f) ? next.delete(f) : next.add(f);
      return next;
    });
  };

  const goPrev = () => pos > 0 && setPos(pos - 1);
  const goNext = () => pos < snapshotIds.length - 1 && setPos(pos + 1);
  const skipCurrent = () => {
    if (!current) return;
    onSkip(current);
    goNext();
  };

  return (
    <div className="organize-full">
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
            title="Rebuild queue from current filters (drops already-processed items)"
          >
            ↻ Rebuild
          </button>
        </div>
        <div className="organize-actions">
          <button onClick={goPrev} disabled={pos === 0}>
            ← Prev
          </button>
          <button
            onClick={skipCurrent}
            disabled={!current}
            className="organize-skip"
            title="Mark as no DLsite match (defer for VNDB later)"
          >
            Skip
          </button>
          <button
            onClick={goNext}
            disabled={pos >= snapshotIds.length - 1}
            className="organize-next"
          >
            Next →
          </button>
        </div>
      </header>

      <div className="organize-body">
        {current ? (
          <div className="organize-detail-wrap">
            <DetailPanel
              item={current}
              autoSearchToken={autoSearchToken}
              onMetadataConfirmed={onMetadataConfirmed}
            />
          </div>
        ) : (
          <div className="organize-empty">
            <h2>Nothing in the queue</h2>
            <p>Toggle filters or click Rebuild after scanning more items.</p>
          </div>
        )}
      </div>
    </div>
  );
}
