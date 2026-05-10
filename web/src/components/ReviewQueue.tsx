import type { InventoryItem } from "../api/types";

export function ReviewQueue({ items }: { items: InventoryItem[] }) {
  const pending = items.filter((item) => item.organizationStatus === "pending");
  return (
    <div className="review-list">
      {pending.map((item) => (
        <article key={item.id}>
          <strong>{item.fileName}</strong>
          <span>{item.legacyLocation ?? "no legacy"}</span>
          <button>Search DLsite</button>
        </article>
      ))}
    </div>
  );
}
