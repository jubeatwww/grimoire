import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  deleteAllMissing,
  excludeInventoryItem,
  fetchLibrary,
  skipInventoryItem,
  triggerScan,
} from "./api/client";
import { invalidateLibrary, useSuggestions, useVocabulary } from "./api/queries";
import type { InventoryItem, Vocabulary } from "./api/types";
import { AppShell } from "./components/AppShell";
import { BrowseMode } from "./components/BrowseMode";
import { ClassifyView } from "./components/ClassifyView";
import { useConfirm } from "./components/ConfirmDialog";
import { DetailPanel } from "./components/DetailPanel";
import { FilterDropdown } from "./components/FilterDropdown";
import { LibraryGrid } from "./components/LibraryGrid";
import { LibraryTable } from "./components/LibraryTable";
import { MoveView } from "./components/MoveView";
import { needsCategory, OrganizeMode, type OrganizeFilter } from "./components/OrganizeMode";
import { ReviewQueue } from "./components/ReviewQueue";
import { SettingsView } from "./components/SettingsView";
import { ThemeToggle } from "./components/ThemeToggle";
import { categoryById, codesMatchQuery } from "./vocabulary";

type ViewMode =
  | "cover"
  | "table"
  | "review"
  | "organize"
  | "browse"
  | "classify"
  | "move"
  | "settings";

export type FilterGroup = "primary" | "workType" | "quick" | "legacy" | "tags" | "codes";
export type Filters = Record<FilterGroup, Set<string>>;

export interface FilterOption {
  id: string;
  count: number;
}

const QUICK_FILTER_LABELS: Record<string, string> = {
  pending: "Pending",
  "has-dlsite": "Has DLsite",
  "has-vndb": "Has VNDB",
  "has-steam": "Has Steam",
  "no-match": "Skipped",
  excluded: "Excluded",
  manual: "Manual entry",
  missing: "Missing file",
  "missing-detail": "Missing detail",
  "missing-cover": "Missing cover",
};
const QUICK_FILTER_OPTIONS: FilterOption[] = [
  { id: "pending", count: 0 },
  { id: "has-dlsite", count: 0 },
  { id: "has-vndb", count: 0 },
  { id: "has-steam", count: 0 },
  { id: "no-match", count: 0 },
  { id: "excluded", count: 0 },
  { id: "manual", count: 0 },
  { id: "missing", count: 0 },
  { id: "missing-detail", count: 0 },
  { id: "missing-cover", count: 0 },
];

function emptyFilters(): Filters {
  return {
    primary: new Set(),
    workType: new Set(),
    quick: new Set(),
    legacy: new Set(),
    tags: new Set(),
    codes: new Set(),
  };
}

function matchesSearch(
  item: InventoryItem,
  query: string,
  vocab: Vocabulary | undefined,
): boolean {
  if (!query) return true;
  const q = query.toLowerCase();
  const haystacks: (string | null | undefined)[] = [
    item.displayTitle,
    item.fileName,
    item.circle,
    item.dlsiteWorkId,
    item.vndbId,
    item.steamAppId,
  ];
  for (const h of haystacks) {
    if (h && h.toLowerCase().includes(q)) return true;
  }
  for (const t of item.sourceTags) {
    if (t.label.toLowerCase().includes(q)) return true;
  }
  if (codesMatchQuery(vocab, item.genreFacets, query)) return true;
  const category = categoryById(vocab, item.primaryCategory);
  if (category && (category.label.includes(query) || category.folderName.toLowerCase().includes(q))) {
    return true;
  }
  return false;
}

/** Distinct source tag labels of an item; the same tag from two sources counts once. */
function tagLabels(item: InventoryItem): string[] {
  return [...new Set(item.sourceTags.map((t) => t.label))];
}

function matches(item: InventoryItem, f: Filters): boolean {
  if (f.primary.size && !f.primary.has(item.primaryCategory ?? "")) return false;
  if (f.workType.size && !f.workType.has(item.workTypeLabel ?? item.workType ?? "")) {
    return false;
  }
  if (f.legacy.size && !f.legacy.has(item.legacyLocation ?? "")) return false;
  if (f.tags.size && !tagLabels(item).some((t) => f.tags.has(t))) return false;
  if (f.codes.size && !item.genreFacets.some((c) => f.codes.has(c))) return false;
  for (const q of f.quick) {
    if (q === "pending" && item.organizationStatus !== "pending") return false;
    if (q === "has-dlsite" && !item.dlsiteWorkId) return false;
    if (q === "has-vndb" && !item.vndbId) return false;
    if (q === "has-steam" && !item.steamAppId) return false;
    if (q === "no-match" && item.organizationStatus !== "no_match") return false;
    if (q === "excluded" && item.organizationStatus !== "ignored") return false;
    if (
      q === "manual" &&
      !(
        item.organizationStatus === "confirmed" &&
        !item.dlsiteWorkId &&
        !item.vndbId &&
        !item.steamAppId
      )
    ) {
      return false;
    }
    if (q === "missing" && !item.missing) return false;
    if (
      q === "missing-detail" &&
      !(item.organizationStatus === "confirmed" && !item.enrichedAt)
    ) {
      return false;
    }
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
  const [search, setSearch] = useState("");
  const [showExcluded, setShowExcluded] = useState(false);
  const [autoSearchToken, setAutoSearchToken] = useState(0);

  const queryClient = useQueryClient();
  const confirm = useConfirm();
  const vocab = useVocabulary();
  const suggestions = useSuggestions().byWork;
  const [organizeFilters, setOrganizeFilters] = useState<OrganizeFilter[] | undefined>();
  const query = useQuery({ queryKey: ["library"], queryFn: fetchLibrary, retry: false });
  const items = query.data?.items ?? [];

  const searchQuery = search.trim().toLowerCase();
  const filteredItems = useMemo(
    () =>
      items.filter((i) => {
        // Hide excluded items unless the user explicitly toggled them on OR
        // is filtering FOR excluded via the Quick filter chip.
        if (
          i.organizationStatus === "ignored" &&
          !showExcluded &&
          !filters.quick.has("excluded")
        ) {
          return false;
        }
        return matches(i, filters) && matchesSearch(i, searchQuery, vocab);
      }),
    [items, filters, searchQuery, showExcluded, vocab],
  );

  const primaryOpts = useMemo(() => countByValue(items, (i) => i.primaryCategory), [items]);
  const workTypeOpts = useMemo(
    () => countByValue(items, (i) => i.workTypeLabel ?? i.workType),
    [items],
  );
  const legacyOpts = useMemo(() => countByValue(items, (i) => i.legacyLocation), [items]);
  const tagsOpts = useMemo(() => countByList(items, tagLabels), [items]);
  const codesOpts = useMemo(() => countByList(items, (i) => i.genreFacets), [items]);
  const primaryLabels = useMemo(
    () => Object.fromEntries((vocab?.categories ?? []).map((c) => [c.id, c.folderName])),
    [vocab],
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
      await invalidateLibrary(queryClient);
    } finally {
      setScanning(false);
    }
  };

  const handleMetadataConfirmed = () => {
    void invalidateLibrary(queryClient);
  };

  const openOrganize = (filters?: OrganizeFilter[]) => {
    setOrganizeFilters(filters);
    setViewMode("organize");
  };

  const handleReviewItem = (item: InventoryItem) => {
    setSelectedId(item.id);
    setAutoSearchToken((t) => t + 1);
  };

  const handleSkipItem = async (item: InventoryItem) => {
    try {
      await skipInventoryItem(item.id);
      void invalidateLibrary(queryClient);
    } catch (e) {
      console.error("skip failed", e);
    }
  };

  const handleExcludeItem = async (item: InventoryItem) => {
    try {
      await excludeInventoryItem(item.id);
      void invalidateLibrary(queryClient);
    } catch (e) {
      console.error("exclude failed", e);
    }
  };

  const missingCount = items.filter((i) => i.missing).length;
  const handleDeleteMissing = async () => {
    const ok = await confirm({
      title: `Delete ${missingCount} missing record${missingCount === 1 ? "" : "s"}?`,
      message:
        "Every inventory row flagged missing gets dropped. Files are already gone from disk; the linked game_works are kept in case other items use them.",
      confirmLabel: `Delete ${missingCount}`,
      danger: true,
    });
    if (!ok) return;
    try {
      await deleteAllMissing();
      void invalidateLibrary(queryClient);
    } catch (e) {
      console.error("delete-missing failed", e);
    }
  };

  const pendingCount = items.filter((i) => i.organizationStatus === "pending").length;
  // Mirrors OrganizeMode's default chip set (pending + needs-detail +
  // needs-category). Excluded / ignored items intentionally do not bump it.
  const organizeCount = items.filter(
    (i) =>
      i.organizationStatus === "pending" ||
      (i.organizationStatus === "confirmed" && !i.enrichedAt) ||
      needsCategory(i, suggestions),
  ).length;
  const classifyCount = [...suggestions.values()].filter((s) => s.confident).length;

  const isOrganize = viewMode === "organize";
  const isBrowse = viewMode === "browse";

  return (
    <AppShell
      chromeless={isOrganize}
      detail={
        isOrganize || isBrowse || viewMode === "settings" || viewMode === "move" ? null : (
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
          onExclude={handleExcludeItem}
          onMetadataConfirmed={handleMetadataConfirmed}
          onExit={() => setViewMode("cover")}
          initialFilters={organizeFilters}
        />
      ) : (
        <>
          <header className="topbar">
            <div className="brand">HG</div>
            <input
              type="search"
              className="topbar-search"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              aria-label="Search title, filename, circle, tag, genre code, DLsite id"
              placeholder="Search title, filename, circle, tag, code (HAG), or DLsite/VNDB ID"
            />
            <button onClick={handleScan} disabled={scanning}>
              {scanning ? "Scanning..." : "Scan"}
            </button>
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
              <button onClick={() => openOrganize()}>
                Organize{organizeCount > 0 ? ` (${organizeCount})` : ""}
              </button>
              <button
                className={viewMode === "classify" ? "active" : ""}
                onClick={() => setViewMode("classify")}
              >
                Classify{classifyCount > 0 ? ` (${classifyCount})` : ""}
              </button>
              <button
                className={viewMode === "move" ? "active" : ""}
                onClick={() => setViewMode("move")}
              >
                Move
              </button>
              <button
                className={viewMode === "settings" ? "active" : ""}
                onClick={() => setViewMode("settings")}
              >
                Settings
              </button>
            </div>
            <div className="filters-bar-divider" />
            <FilterDropdown
              label="Primary"
              options={primaryOpts}
              selected={filters.primary}
              onToggle={(v) => toggleFilter("primary", v)}
              labels={primaryLabels}
            />
            <FilterDropdown
              label="Codes"
              options={codesOpts}
              selected={filters.codes}
              onToggle={(v) => toggleFilter("codes", v)}
              searchable
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
            <button
              type="button"
              className={`filter-trigger filter-toggle ${showExcluded ? "active" : ""}`}
              onClick={() => setShowExcluded((v) => !v)}
              title={
                showExcluded
                  ? "Hide excluded (not-a-game) items"
                  : "Show excluded (not-a-game) items"
              }
            >
              <span>{showExcluded ? "● Excluded shown" : "○ Excluded hidden"}</span>
            </button>
            {missingCount > 0 && (
              <button
                type="button"
                className="filter-trigger filter-delete-missing"
                onClick={handleDeleteMissing}
                title="Permanently delete every inventory record whose file is gone"
              >
                🗑 Delete {missingCount} missing
              </button>
            )}
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
          {viewMode === "settings" && <SettingsView />}
          {viewMode === "move" && <MoveView />}
          {viewMode === "classify" && (
            <ClassifyView
              items={items}
              selectedId={selectedItem?.id ?? null}
              onSelect={(item) => setSelectedId(item.id)}
              onReviewRest={() => openOrganize(["needs-category"])}
            />
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
