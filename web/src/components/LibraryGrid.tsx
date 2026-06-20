import type { InventoryItem } from "../api/types";
import { useImagePreview } from "./useImagePreview";

interface LibraryGridProps {
  items: InventoryItem[];
  selectedId: string | null;
  onSelect: (item: InventoryItem) => void;
}

export function LibraryGrid({ items, selectedId, onSelect }: LibraryGridProps) {
  const { hoverProps, preview } = useImagePreview();
  return (
    <div className="library-grid">
      {items.map((item, index) => (
        <button
          className={`game-card ${selectedId === item.id ? "selected" : ""}`}
          key={item.id}
          onClick={() => onSelect(item)}
          {...hoverProps(item.coverImageUrl)}
        >
          {item.coverImageUrl ? (
            <div className="cover cover-image">
              <img src={item.coverImageUrl} alt="" loading="lazy" />
              <span>{item.organizationStatus}</span>
            </div>
          ) : (
            <div className={`cover cover-${index % 6}`}>
              <span>{item.organizationStatus}</span>
            </div>
          )}
          <strong>{item.displayTitle ?? item.fileName}</strong>
          <small>{item.primaryCategory ?? "Unsorted"} · {item.legacyLocation ?? "no legacy"}</small>
          <div className="mini-tags">
            {item.genreFacets.map((facet) => <span key={facet}>{facet}</span>)}
          </div>
        </button>
      ))}
      {preview}
    </div>
  );
}
