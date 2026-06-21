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

export async function excludeInventoryItem(itemId: string): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/exclude`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId }),
  });
  if (!r.ok) throw new Error(`Exclude failed: ${r.status}`);
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
    if (r.status === 400) {
      throw new Error(
        "Couldn't recognise a Steam URL, VNDB id/URL, or DLsite RJ/VJ/BJ code in that input",
      );
    }
    if (r.status === 404) {
      throw new Error("Source has no record for that id — double-check the link");
    }
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
    coverImageUrl?: string;
    previewImageUrls?: string[];
  },
): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/edit-work`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId, ...fields }),
  });
  if (!r.ok) throw new Error(`Edit failed: ${r.status}`);
}

export async function deleteInventoryItem(itemId: string): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/delete-item`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId }),
  });
  if (!r.ok) throw new Error(`Delete failed: ${r.status}`);
}

export async function deleteAllMissing(): Promise<number> {
  const r = await fetch(`${API_BASE}/api/metadata/delete-missing`, {
    method: "POST",
  });
  if (!r.ok) throw new Error(`Delete-missing failed: ${r.status}`);
  const data: { deleted: number } = await r.json();
  return data.deleted;
}

export async function createManualEntry(itemId: string): Promise<void> {
  const r = await fetch(`${API_BASE}/api/metadata/manual`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId }),
  });
  if (!r.ok) throw new Error(`Manual entry creation failed: ${r.status}`);
}

export async function uploadAsset(file: File): Promise<string> {
  const form = new FormData();
  form.append("file", file);
  const r = await fetch(`${API_BASE}/api/assets/upload`, {
    method: "POST",
    body: form,
  });
  if (!r.ok) {
    if (r.status === 415) throw new Error("Unsupported image type");
    if (r.status === 413) throw new Error("File too large (max 20MB)");
    throw new Error(`Upload failed: ${r.status}`);
  }
  const data: { url: string } = await r.json();
  return data.url;
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
  source: "dlsite" | "vndb" | "steam",
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
