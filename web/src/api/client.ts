import type { LibraryResponse } from "./types";

const API_BASE = import.meta.env.VITE_API_BASE ?? "";

export async function fetchLibrary(): Promise<LibraryResponse> {
  const response = await fetch(`${API_BASE}/api/library/`);
  if (!response.ok) {
    throw new Error(`Library request failed: ${response.status}`);
  }
  return response.json();
}

export function downloadUrl(itemId: string): string {
  return `${API_BASE}/api/downloads/${itemId}`;
}
