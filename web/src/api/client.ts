import type { ConfirmResponse, LibraryResponse, SearchResponse } from "./types";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";

export async function fetchLibrary(): Promise<LibraryResponse> {
  const response = await fetch(`${API_BASE}/api/library/`);
  if (!response.ok) {
    throw new Error(`Library request failed: ${response.status}`);
  }
  return response.json();
}

export async function searchMetadata(query: string): Promise<SearchResponse> {
  const response = await fetch(`${API_BASE}/api/metadata/search`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ query }),
  });
  if (!response.ok) {
    throw new Error(`Metadata search failed: ${response.status}`);
  }
  return response.json();
}

export async function confirmCandidate(
  candidateId: string,
  inventoryItemId: string,
): Promise<ConfirmResponse> {
  const response = await fetch(`${API_BASE}/api/metadata/confirm`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ candidateId, inventoryItemId }),
  });
  if (!response.ok) {
    throw new Error(`Confirm failed: ${response.status}`);
  }
  return response.json();
}

export async function triggerScan(): Promise<{ scanned: number; warnings: string[] }> {
  const response = await fetch(`${API_BASE}/api/scan`, { method: "POST" });
  if (!response.ok) {
    throw new Error(`Scan failed: ${response.status}`);
  }
  return response.json();
}

export async function skipInventoryItem(itemId: string): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/skip`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId }),
  });
  if (!r.ok) throw new Error(`Skip failed: ${r.status}`);
}

export async function linkInventoryItem(
  itemId: string,
  worknoOrUrl: string,
): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/link`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId, worknoOrUrl }),
  });
  if (!r.ok) {
    if (r.status === 400) throw new Error("Couldn't find an RJ/VJ/BJ code in that input");
    if (r.status === 404) throw new Error("DLsite has no record for that workno");
    throw new Error(`Link failed: ${r.status}`);
  }
}

export async function resetInventoryItem(itemId: string): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/reset`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId }),
  });
  if (!r.ok) throw new Error(`Reset failed: ${r.status}`);
}

export async function editWork(
  itemId: string,
  fields: {
    displayTitle?: string;
    workType?: string;
    workTypeLabel?: string;
  },
): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/edit-work`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId, ...fields }),
  });
  if (!r.ok) throw new Error(`Edit failed: ${r.status}`);
}

export async function editInventoryItem(
  itemId: string,
  fields: { primaryCategory?: string },
): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/edit-item`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId, ...fields }),
  });
  if (!r.ok) throw new Error(`Edit failed: ${r.status}`);
}

export async function refreshMetadata(
  itemId: string,
  source: "dlsite" | "vndb",
): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/refresh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId, source }),
  });
  if (!r.ok) throw new Error(`Refresh failed: ${r.status}`);
}

export function downloadUrl(itemId: string): string {
  return `${API_BASE}/api/downloads/${itemId}`;
}
