import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { fetchLibrary } from "./api/client";
import type { InventoryItem } from "./api/types";
import { AppShell } from "./components/AppShell";
import { DetailPanel } from "./components/DetailPanel";
import { LibraryGrid } from "./components/LibraryGrid";
import { LibraryTable } from "./components/LibraryTable";
import { ReviewQueue } from "./components/ReviewQueue";

type ViewMode = "cover" | "table" | "review";

export function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("cover");
  const [selected, setSelected] = useState<InventoryItem | null>(null);
  const query = useQuery({ queryKey: ["library"], queryFn: fetchLibrary, retry: false });
  const items = query.data?.items ?? [];
  const selectedItem = useMemo(() => selected ?? items[0] ?? null, [items, selected]);

  return (
    <AppShell detail={<DetailPanel item={selectedItem} />}>
      <header className="topbar">
        <input aria-label="Search title, filename, circle, tag, DLsite id" />
        <button>Scan</button>
        <button className="primary">Import</button>
      </header>
      <div className="view-switch">
        <button className={viewMode === "cover" ? "active" : ""} onClick={() => setViewMode("cover")}>Cover</button>
        <button className={viewMode === "table" ? "active" : ""} onClick={() => setViewMode("table")}>Table</button>
        <button className={viewMode === "review" ? "active" : ""} onClick={() => setViewMode("review")}>Review Queue</button>
      </div>
      {viewMode === "cover" && <LibraryGrid items={items} selectedId={selectedItem?.id ?? null} onSelect={setSelected} />}
      {viewMode === "table" && <LibraryTable items={items} />}
      {viewMode === "review" && <ReviewQueue items={items} />}
    </AppShell>
  );
}
