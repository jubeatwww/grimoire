import type { InventoryItem } from "../api/types";

export function LibraryTable({ items }: { items: InventoryItem[] }) {
  return (
    <table className="library-table">
      <thead>
        <tr>
          <th>Name</th>
          <th>Primary</th>
          <th>Facets</th>
          <th>Legacy</th>
          <th>Status</th>
        </tr>
      </thead>
      <tbody>
        {items.map((item) => (
          <tr key={item.id}>
            <td>{item.fileName}</td>
            <td>{item.primaryCategory ?? "Unsorted"}</td>
            <td>{item.genreFacets.join(", ")}</td>
            <td>{item.legacyLocation ?? ""}</td>
            <td>{item.organizationStatus}</td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
