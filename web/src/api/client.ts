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

export function downloadUrl(itemId: string): string {
  return `${API_BASE}/api/downloads/${itemId}`;
}
