import { downloadUrl } from "../api/client";
import type { InventoryItem } from "../api/types";

interface DetailPanelProps {
  item: InventoryItem | null;
}

export function DetailPanel({ item }: DetailPanelProps) {
  if (!item) {
    return <div className="empty-detail">Select a game</div>;
  }

  return (
    <div className="detail">
      <div className="large-cover" />
      <h2>{item.fileName}</h2>
      <p>{item.primaryCategory ?? "Unsorted"} · {item.organizationStatus}</p>
      <dl>
        <dt>Legacy location</dt>
        <dd>{item.legacyLocation ?? "none"}</dd>
        <dt>Version</dt>
        <dd>{item.version ?? "unknown"}</dd>
        <dt>Language</dt>
        <dd>{item.language ?? "unknown"}</dd>
      </dl>
      <div className="detail-actions">
        <button>Search DLsite</button>
        <a className="button" href={downloadUrl(item.id)}>Download</a>
      </div>
    </div>
  );
}
