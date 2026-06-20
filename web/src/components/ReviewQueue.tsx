import type { InventoryItem } from "../api/types";

interface ReviewQueueProps {
  items: InventoryItem[];
  onReview: (item: InventoryItem) => void;
}

const RJ_RE = /(?:RJ|VJ|BJ)\d{6,8}/i;

function extractRj(name: string): string | null {
  return name.match(RJ_RE)?.[0]?.toUpperCase() ?? null;
}

export function ReviewQueue({ items, onReview }: ReviewQueueProps) {
  const pending = items.filter((i) => i.organizationStatus === "pending");

  if (pending.length === 0) {
    return (
      <div className="review-empty">
        <h2>Nothing to review</h2>
        <p>All inventory items have been matched or confirmed.</p>
      </div>
    );
  }

  return (
    <div className="review-queue">
      <header className="review-header">
        <h2>Pending metadata</h2>
        <p>
          {pending.length} item{pending.length === 1 ? "" : "s"} need a DLsite match.
          Click <em>Review</em> to load the item and run a search.
        </p>
      </header>
      <ul className="review-list">
        {pending.map((item) => {
          const rj = extractRj(item.fileName);
          return (
            <li key={item.id} className="review-card">
              <div className="review-thumb">
                {item.coverImageUrl ? (
                  <img src={item.coverImageUrl} alt="" loading="lazy" />
                ) : (
                  <div className="thumb-fallback" />
                )}
              </div>
              <div className="review-body">
                <strong title={item.fileName}>{item.fileName}</strong>
                <div className="review-meta">
                  {rj && <code className="review-rj">{rj}</code>}
                  {item.legacyLocation && <span>{item.legacyLocation}</span>}
                  {item.primaryCategory && <span>{item.primaryCategory}</span>}
                </div>
              </div>
              <button className="review-action" onClick={() => onReview(item)}>
                Review →
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}
