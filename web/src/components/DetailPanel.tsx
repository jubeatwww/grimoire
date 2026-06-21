import type { InventoryItem } from "../api/types";
import { ItemFocus } from "./ItemFocus";
import { SearchPanel } from "./SearchPanel";

interface DetailPanelProps {
  item: InventoryItem | null;
  autoSearchToken?: number;
  onMetadataConfirmed?: () => void;
}

/// Default detail-pane composition: focus block above, search/candidates below.
/// OrganizeMode uses ItemFocus + SearchPanel directly to lay them out in
/// separate columns.
export function DetailPanel({ item, autoSearchToken, onMetadataConfirmed }: DetailPanelProps) {
  if (!item) {
    return <div className="empty-detail">Select a game</div>;
  }
  return (
    <div className="detail">
      <ItemFocus item={item} onChanged={onMetadataConfirmed} />
      <SearchPanel
        item={item}
        autoSearchToken={autoSearchToken}
        onChanged={onMetadataConfirmed}
      />
    </div>
  );
}
