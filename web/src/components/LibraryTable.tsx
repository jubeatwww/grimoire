import { useState } from "react";
import { useVocabulary } from "../api/queries";
import type { InventoryItem } from "../api/types";
import { categoryName } from "../vocabulary";

type SortKey = "name" | "status" | "primary" | "circle" | "workType";

interface LibraryTableProps {
  items: InventoryItem[];
  selectedId: string | null;
  onSelect: (item: InventoryItem) => void;
}

const STATUS_RANK: Record<string, number> = {
  pending: 0,
  matched: 1,
  confirmed: 2,
  ignored: 3,
};

/** Category id → folder sort order, so the Primary column sorts like the NAS. */
type CategoryRank = (id: string) => number;

function compare(
  a: InventoryItem,
  b: InventoryItem,
  key: SortKey,
  dir: 1 | -1,
  rank: CategoryRank,
): number {
  const av = sortValue(a, key, rank);
  const bv = sortValue(b, key, rank);
  if (av < bv) return -1 * dir;
  if (av > bv) return 1 * dir;
  return 0;
}

function sortValue(item: InventoryItem, key: SortKey, rank: CategoryRank): string | number {
  switch (key) {
    case "name":
      return (item.displayTitle ?? item.fileName).toLowerCase();
    case "status":
      return STATUS_RANK[item.organizationStatus] ?? 99;
    case "primary":
      return item.primaryCategory ? rank(item.primaryCategory) : Number.MAX_SAFE_INTEGER;
    case "circle":
      return ((item.displayTitle ? "" : "~") + (item.fileName)).toLowerCase();
    case "workType":
      return (item.workType ?? "~").toLowerCase();
  }
}

export function LibraryTable({ items, selectedId, onSelect }: LibraryTableProps) {
  const [sortKey, setSortKey] = useState<SortKey>("name");
  const [sortDir, setSortDir] = useState<1 | -1>(1);
  const vocab = useVocabulary();
  const rank: CategoryRank = (id) =>
    vocab?.categories.find((c) => c.id === id)?.sortOrder ?? Number.MAX_SAFE_INTEGER - 1;

  const sorted = [...items].sort((a, b) => compare(a, b, sortKey, sortDir, rank));

  const handleSort = (key: SortKey) => {
    if (sortKey === key) setSortDir((d) => (d === 1 ? -1 : 1));
    else {
      setSortKey(key);
      setSortDir(1);
    }
  };

  const arrow = (key: SortKey) =>
    sortKey === key ? <span className="sort-arrow">{sortDir === 1 ? "▲" : "▼"}</span> : null;

  return (
    <div className="library-table-wrap">
      <table className="library-table">
        <thead>
          <tr>
            <th className="col-cover" />
            <th>
              <button onClick={() => handleSort("name")}>Title{arrow("name")}</button>
            </th>
            <th>
              <button onClick={() => handleSort("status")}>Status{arrow("status")}</button>
            </th>
            <th>
              <button onClick={() => handleSort("primary")}>Primary{arrow("primary")}</button>
            </th>
            <th>
              <button onClick={() => handleSort("workType")}>Type{arrow("workType")}</button>
            </th>
            <th>Legacy</th>
            <th>Codes</th>
          </tr>
        </thead>
        <tbody>
          {sorted.map((item) => (
            <tr
              key={item.id}
              className={selectedId === item.id ? "selected" : ""}
              onClick={() => onSelect(item)}
            >
              <td className="col-cover">
                {item.coverImageUrl ? (
                  <img src={item.coverImageUrl} alt="" loading="lazy" />
                ) : (
                  <div className="thumb-fallback" />
                )}
              </td>
              <td className="col-title">
                <strong>{item.displayTitle ?? item.fileName}</strong>
                {item.displayTitle && <small>{item.fileName}</small>}
              </td>
              <td>
                <span className={`status-pill status-${item.organizationStatus}`}>
                  {item.organizationStatus}
                </span>
              </td>
              <td>
                {item.primaryCategory ? (
                  <span className={item.categoryConfirmed ? "" : "unconfirmed"}>
                    {categoryName(vocab, item.primaryCategory)}
                  </span>
                ) : (
                  <em>Unsorted</em>
                )}
              </td>
              <td>
                {item.workTypeLabel || item.workType ? (
                  <span className="work-type-pill" title={item.workType ?? undefined}>
                    {item.workTypeLabel ?? item.workType}
                  </span>
                ) : (
                  <em>—</em>
                )}
              </td>
              <td>{item.legacyLocation ?? <em>—</em>}</td>
              <td className="col-facets">
                {item.genreFacets.length > 0 ? item.genreFacets.join(", ") : <em>—</em>}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {sorted.length === 0 && <p className="empty-state">No items match the current filters.</p>}
    </div>
  );
}
