import { useEffect, useRef, useState } from "react";
import type { FilterOption } from "../App";

interface FilterDropdownProps {
  label: string;
  options: FilterOption[];
  selected: Set<string>;
  onToggle: (value: string) => void;
  /** Show search input inside the popover (Tags). */
  searchable?: boolean;
  /** Render a wider popover for sections with many chips. */
  wide?: boolean;
  /** Optional human labels for options whose `id` is a slug (Quick filters). */
  labels?: Record<string, string>;
  /** Hide the count badge on each option (Quick filters). */
  hideCounts?: boolean;
  /** When > 0, collapse the non-selected list to this many entries with a
   *  "Show more" button. Selected options stay pinned at top. */
  initialLimit?: number;
}

export function FilterDropdown({
  label,
  options,
  selected,
  onToggle,
  searchable,
  wide,
  labels,
  hideCounts,
  initialLimit,
}: FilterDropdownProps) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [expanded, setExpanded] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const onMouseDown = (e: MouseEvent) => {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onMouseDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onMouseDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  // Reset search when closing so reopening starts fresh.
  useEffect(() => {
    if (!open) setSearch("");
  }, [open]);

  // Selected options always visible; remaining filtered + collapsed.
  const selectedOpts = options.filter((o) => selected.has(o.id));
  const restOpts = options.filter((o) => !selected.has(o.id));
  const q = search.trim().toLowerCase();
  const restFiltered = q
    ? restOpts.filter((o) => o.id.toLowerCase().includes(q))
    : restOpts;
  const restVisible =
    initialLimit && !expanded && !q
      ? restFiltered.slice(0, initialLimit)
      : restFiltered;
  const hiddenCount = restFiltered.length - restVisible.length;

  const labelFor = (o: FilterOption) => labels?.[o.id] ?? o.id;

  const count = selected.size;
  const empty = options.length === 0;

  return (
    <div ref={rootRef} className="filter-dropdown">
      <button
        type="button"
        className={`filter-trigger ${open ? "open" : ""} ${count > 0 ? "active" : ""}`}
        onClick={() => setOpen((o) => !o)}
        disabled={empty && count === 0}
        aria-expanded={open}
      >
        <span>{label}</span>
        {count > 0 && <span className="filter-trigger-badge">{count}</span>}
        <span className="filter-trigger-caret" aria-hidden>
          ▾
        </span>
      </button>
      {open && (
        <div
          className={`filter-popover ${wide ? "filter-popover-wide" : ""}`}
          role="dialog"
        >
          {searchable && (
            <input
              autoFocus
              className="filter-popover-search"
              type="search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              placeholder={`Filter ${label.toLowerCase()}…`}
            />
          )}
          <div className="filter-popover-chips">
            {selectedOpts.map((o) => (
              <button
                key={o.id}
                type="button"
                className="chip chip-on"
                onClick={() => onToggle(o.id)}
                title={o.id}
              >
                {labelFor(o)}
                {!hideCounts && <span className="chip-count">{o.count}</span>}
              </button>
            ))}
            {restVisible.map((o) => (
              <button
                key={o.id}
                type="button"
                className="chip"
                onClick={() => onToggle(o.id)}
                title={o.id}
              >
                {labelFor(o)}
                {!hideCounts && <span className="chip-count">{o.count}</span>}
              </button>
            ))}
            {selectedOpts.length === 0 && restVisible.length === 0 && (
              <p className="filter-popover-empty">No matches.</p>
            )}
          </div>
          {!q && hiddenCount > 0 && (
            <button
              type="button"
              className="filter-popover-expand"
              onClick={() => setExpanded((x) => !x)}
            >
              {expanded ? "Show less" : `Show ${hiddenCount} more`}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
