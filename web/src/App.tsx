import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { fetchLibrary, skipInventoryItem, triggerScan } from "./api/client";
import type { InventoryItem } from "./api/types";
import { AppShell } from "./components/AppShell";
import { BrowseMode } from "./components/BrowseMode";
import { DetailPanel } from "./components/DetailPanel";
import { FilterDropdown } from "./components/FilterDropdown";
import { LibraryGrid } from "./components/LibraryGrid";
import { LibraryTable } from "./components/LibraryTable";
import { OrganizeMode } from "./components/OrganizeMode";
import { ReviewQueue } from "./components/ReviewQueue";
import { ThemeToggle } from "./components/ThemeToggle";

type ViewMode = "cover" | "table" | "review" | "organize" | "browse";

export type FilterGroup = "primary" | "workType" | "quick" | "legacy" | "tags";
export type Filters = Record<FilterGroup, Set<string>>;

export interface FilterOption {
  id: string;
  count: number;
}

export const PRIMARY_CATEGORIES = [
  "Visual Novel",
  "Action",
  "RPG",
  "Simulation",
  "Strategy",
  "3D",
];

const QUICK_FILTER_LABELS: Record<string, string> = {
  "needs-review": "Needs review",
  "has-dlsite": "Has DLsite",
  "missing-cover": "Missing cover",
};
const QUICK_FILTER_OPTIONS: FilterOption[] = [
  { id: "needs-review", count: 0 },
  { id: "has-dlsite", count: 0 },
  { id: "missing-cover", count: 0 },
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

  const primaryOpts = useMemo(() => countByValue(items, (i) => i.primaryCategory), [items]);
  const workTypeOpts = useMemo(
    () => countByValue(items, (i) => i.workTypeLabel ?? i.workType),
    [items],
  );
  const legacyOpts = useMemo(() => countByValue(items, (i) => i.legacyLocation), [items]);
  const tagsOpts = useMemo(() => countByList(items, (i) => i.sourceTags), [items]);

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
  const isBrowse = viewMode === "browse";

  return (
    <AppShell
      chromeless={isOrganize}
      detail={
        isOrganize || isBrowse ? null : (
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
            <div className="brand">HG</div>
            <nav className="topbar-nav">
              <button className="active">作品庫</button>
              <button>匯入 staging</button>
              <button>下載紀錄</button>
              <button>設定</button>
            </nav>
            <input
              type="search"
              className="topbar-search"
              aria-label="Search title, filename, circle, tag, DLsite id"
              placeholder="Search title, filename, circle, tag, or DLsite ID"
            />
            <button onClick={handleScan} disabled={scanning}>
              {scanning ? "Scanning..." : "Scan"}
            </button>
            <button className="primary">Import</button>
            <ThemeToggle />
          </header>
          <div className="filters-bar">
            <div className="view-switch">
              <button
                className={viewMode === "cover" ? "active" : ""}
                onClick={() => setViewMode("cover")}
              >
                Cover
              </button>
              <button
                className={viewMode === "browse" ? "active" : ""}
                onClick={() => setViewMode("browse")}
              >
                Browse
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
            <div className="filters-bar-divider" />
            <FilterDropdown
              label="Primary"
              options={primaryOpts}
              selected={filters.primary}
              onToggle={(v) => toggleFilter("primary", v)}
            />
            <FilterDropdown
              label="Work type"
              options={workTypeOpts}
              selected={filters.workType}
              onToggle={(v) => toggleFilter("workType", v)}
            />
            <FilterDropdown
              label="Tags"
              options={tagsOpts}
              selected={filters.tags}
              onToggle={(v) => toggleFilter("tags", v)}
              searchable
              wide
              initialLimit={30}
            />
            <FilterDropdown
              label="Legacy"
              options={legacyOpts}
              selected={filters.legacy}
              onToggle={(v) => toggleFilter("legacy", v)}
            />
            <FilterDropdown
              label="Quick"
              options={QUICK_FILTER_OPTIONS}
              selected={filters.quick}
              onToggle={(v) => toggleFilter("quick", v)}
              labels={QUICK_FILTER_LABELS}
              hideCounts
            />
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
          {viewMode === "browse" && (
            <BrowseMode
              items={filteredItems}
              selectedId={selectedItem?.id ?? null}
              autoSearchToken={autoSearchToken}
              onSelect={(item) => setSelectedId(item.id)}
              onMetadataConfirmed={handleMetadataConfirmed}
            />
          )}
        </>
      )}
    </AppShell>
  );
}
