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

type ViewMode = "cover" | "table" | "review" | "organize";

export type FilterGroup = "primary" | "quick" | "legacy";
export type Filters = Record<FilterGroup, Set<string>>;

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
export const LEGACY_LOCATIONS = ["ADV", "ACT", "RPG", "舊 SIM+SLG", "未分類"];

function emptyFilters(): Filters {
  return { primary: new Set(), quick: new Set(), legacy: new Set() };
}

function matches(item: InventoryItem, f: Filters): boolean {
  if (f.primary.size && !f.primary.has(item.primaryCategory ?? "")) return false;
  if (f.legacy.size && !f.legacy.has(item.legacyLocation ?? "")) return false;
  for (const q of f.quick) {
    if (q === "needs-review" && item.organizationStatus !== "pending") return false;
    if (q === "has-dlsite" && !item.displayTitle) return false;
    if (q === "missing-cover" && item.coverImageUrl) return false;
  }
  return true;
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
