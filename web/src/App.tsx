import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { fetchLibrary, triggerScan } from "./api/client";
import { AppShell } from "./components/AppShell";
import { DetailPanel } from "./components/DetailPanel";
import { LibraryGrid } from "./components/LibraryGrid";
import { LibraryTable } from "./components/LibraryTable";
import { ReviewQueue } from "./components/ReviewQueue";

type ViewMode = "cover" | "table" | "review";

export function App() {
  const [viewMode, setViewMode] = useState<ViewMode>("cover");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [scanning, setScanning] = useState(false);
  const queryClient = useQueryClient();
  const query = useQuery({ queryKey: ["library"], queryFn: fetchLibrary, retry: false });
  const items = query.data?.items ?? [];
  const selectedItem = useMemo(
    () => items.find((i) => i.id === selectedId) ?? items[0] ?? null,
    [items, selectedId],
  );

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

  return (
    <AppShell detail={<DetailPanel item={selectedItem} onMetadataConfirmed={handleMetadataConfirmed} />}>
      <header className="topbar">
        <input aria-label="Search title, filename, circle, tag, DLsite id" />
        <button onClick={handleScan} disabled={scanning}>{scanning ? "Scanning..." : "Scan"}</button>
        <button className="primary">Import</button>
      </header>
      <div className="view-switch">
        <button className={viewMode === "cover" ? "active" : ""} onClick={() => setViewMode("cover")}>Cover</button>
        <button className={viewMode === "table" ? "active" : ""} onClick={() => setViewMode("table")}>Table</button>
        <button className={viewMode === "review" ? "active" : ""} onClick={() => setViewMode("review")}>Review Queue</button>
      </div>
      {viewMode === "cover" && <LibraryGrid items={items} selectedId={selectedItem?.id ?? null} onSelect={(item) => setSelectedId(item.id)} />}
      {viewMode === "table" && <LibraryTable items={items} />}
      {viewMode === "review" && <ReviewQueue items={items} />}
    </AppShell>
  );
}
