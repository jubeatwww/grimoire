import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { fetchLibrary, skipInventoryItem, triggerScan } from "./api/client";
import type { InventoryItem } from "./api/types";
import { AppShell } from "./components/AppShell";
import { DetailPanel } from "./components/DetailPanel";
import { LibraryGrid } from "./components/LibraryGrid";
import { LibraryTable } from "./components/LibraryTable";
import { OrganizeMode } from "./components/OrganizeMode";
import { ReviewQueue } from "./components/ReviewQueue";
import { ThemeToggle } from "./components/ThemeToggle";

type ViewMode = "cover" | "table" | "review" | "organize";

export type FilterGroup = "primary" | "workType" | "quick" | "legacy" | "tags";
export type Filters = Record<FilterGroup, Set<string>>;

export interface FilterOption {
  id: string;
  count: number;
}
export interface FilterOptions {
  primary: FilterOption[];
  workType: FilterOption[];
  legacy: FilterOption[];
  tags: FilterOption[];
}

/// Hardcoded list for the primary-category SELECT dropdown when editing an
/// item — gives the user something to pick from on a fresh library. Sidebar
/// filter chips no longer use this; they derive from actual item data.
export const PRIMARY_CATEGORIES = [
  "Visual Novel",
  "Action",
  "RPG",
  "Simulation",
  "Strategy",
  "3D",
];

export const QUICK_FILTERS: { id: string; label: string }[] = [
  { id: "needs-review", label: "Needs review" },
  { id: "has-dlsite", label: "Has DLsite" },
  { id: "missing-cover", label: "Missing cover" },
];

function emptyFilters(): Filters {
  return {
    primary: new Set(),
    workType: new Set(),
    quick: new Set(),
    legacy: new Set(),
    tags: new Set(),
  };
}

function matches(item: InventoryItem, f: Filters): boolean {
  if (f.primary.size && !f.primary.has(item.primaryCategory ?? "")) return false;
  if (f.workType.size && !f.workType.has(item.workTypeLabel ?? item.workType ?? "")) {
    return false;
  }
  if (f.legacy.size && !f.legacy.has(item.legacyLocation ?? "")) return false;
  if (f.tags.size) {
    const itemTags = item.sourceTags ?? [];
    let any = false;
    for (const t of itemTags) {
      if (f.tags.has(t)) {
        any = true;
        break;
      }
    }
    if (!any) return false;
  }
  for (const q of f.quick) {
    if (q === "needs-review" && item.organizationStatus !== "pending") return false;
    if (q === "has-dlsite" && !item.displayTitle) return false;
    if (q === "missing-cover" && item.coverImageUrl) return false;
  }
  return true;
}

function countByValue<T>(
  items: T[],
  getter: (item: T) => string | null | undefined,
): FilterOption[] {
  const m = new Map<string, number>();
  for (const item of items) {
    const v = getter(item);
    if (!v) continue;
    m.set(v, (m.get(v) ?? 0) + 1);
  }
  return [...m.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([id, count]) => ({ id, count }));
}

function countByList<T>(
  items: T[],
  getter: (item: T) => string[] | null | undefined,
): FilterOption[] {
  const m = new Map<string, number>();
  for (const item of items) {
    const list = getter(item);
    if (!list) continue;
    for (const v of list) m.set(v, (m.get(v) ?? 0) + 1);
  }
  return [...m.entries()]
    .sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))
    .map(([id, count]) => ({ id, count }));
}

export function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("cover");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const [filters, setFilters] = useState<Filters>(emptyFilters);
  const [autoSearchToken, setAutoSearchToken] = useState(0);

  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["library"], queryFn: fetchLibrary, retry: false });
  const items = query.data?.items ?? [];

  const filteredItems = useMemo(
    () => items.filter((i) => matches(i, filters)),
    [items, filters],
  );

  const options = useMemo<FilterOptions>(
    () => ({
      primary: countByValue(items, (i) => i.primaryCategory),
      workType: countByValue(items, (i) => i.workTypeLabel ?? i.workType),
      legacy: countByValue(items, (i) => i.legacyLocation),
      tags: countByList(items, (i) => i.sourceTags),
    }),
    [items],
  );

  const selectedItem = useMemo(
    () =>
      items.find((i) => i.id === selectedId) ??
      filteredItems[0] ??
      items[0] ??
      null,
    [items, filteredItems, selectedId],
  );

  const toggleFilter = (group: FilterGroup, value: string) => {
    setFilters((prev) => {
      const next = new Set(prev[group]);
      next.has(value) ? next.delete(value) : next.add(value);
      return { ...prev, [group]: next };
    });
  };

  const handleScan = async () => {
    setScanning(true);
    try {
      await triggerScan();
      await queryClient.invalidateQueries({ queryKey: ["library"] });
    } finally {
      setScanning(false);
    }
  };

  const handleMetadataConfirmed = () => {
    queryClient.invalidateQueries({ queryKey: ["library"] });
  };

  const handleReviewItem = (item: InventoryItem) => {
    setSelectedId(item.id);
    setAutoSearchToken((t) => t + 1);
  };

  const handleSkipItem = async (item: InventoryItem) => {
    try {
      await skipInventoryItem(item.id);
      queryClient.invalidateQueries({ queryKey: ["library"] });
    } catch (e) {
      console.error("skip failed", e);
    }
  };

  const pendingCount = items.filter((i) => i.organizationStatus === "pending").length;
  const organizeCount = items.filter(
    (i) =>
      i.organizationStatus === "pending" ||
      (i.organizationStatus === "confirmed" && !i.description),
  ).length;

  const isOrganize = viewMode === "organize";

  return (
    <AppShell
      filters={filters}
      options={options}
      onToggleFilter={toggleFilter}
      chromeless={isOrganize}
      detail={
        isOrganize ? null : (
          <DetailPanel
            item={selectedItem}
            autoSearchToken={autoSearchToken}
            onMetadataConfirmed={handleMetadataConfirmed}
          />
        )
      }
    >
      {isOrganize ? (
        <OrganizeMode
          items={items}
          autoSearchToken={autoSearchToken}
          onAutoTrigger={() => setAutoSearchToken((t) => t + 1)}
          onSkip={handleSkipItem}
          onMetadataConfirmed={handleMetadataConfirmed}
          onExit={() => setViewMode("cover")}
        />
      ) : (
        <>
          <header className="topbar">
            <input
              type="search"
              aria-label="Search title, filename, circle, tag, DLsite id"
              placeholder="Search title, filename, circle, tag, or DLsite ID"
            />
            <button onClick={handleScan} disabled={scanning}>
              {scanning ? "Scanning..." : "Scan"}
            </button>
            <button className="primary">Import</button>
            <ThemeToggle />
          </header>
          <div className="view-switch">
            <button
              className={viewMode === "cover" ? "active" : ""}
              onClick={() => setViewMode("cover")}
            >
              Cover
            </button>
            <button
              className={viewMode === "table" ? "active" : ""}
              onClick={() => setViewMode("table")}
            >
              Table
            </button>
            <button
              className={viewMode === "review" ? "active" : ""}
              onClick={() => setViewMode("review")}
            >
              Review Queue{pendingCount > 0 ? ` (${pendingCount})` : ""}
            </button>
            <button onClick={() => setViewMode("organize")}>
              Organize{organizeCount > 0 ? ` (${organizeCount})` : ""}
            </button>
          </div>
          {viewMode === "cover" && (
            <LibraryGrid
              items={filteredItems}
              selectedId={selectedItem?.id ?? null}
              onSelect={(item) => setSelectedId(item.id)}
            />
          )}
          {viewMode === "table" && (
            <LibraryTable
              items={filteredItems}
              selectedId={selectedItem?.id ?? null}
              onSelect={(item) => setSelectedId(item.id)}
            />
          )}
          {viewMode === "review" && (
            <ReviewQueue items={items} onReview={handleReviewItem} />
          )}
        </>
      )}
    </AppShell>
  );
}
