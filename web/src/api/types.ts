export type OrganizationStatus = "pending" | "matched" | "confirmed" | "ignored";
export type PlayStatus = "not_played" | "want_to_play" | "playing" | "completed" | "dropped";

export interface InventoryItem {
  id: string;
  sourceId: string;
  fileName: string;
  legacyLocation: string | null;
  primaryCategory: string | null;
  genreFacets: string[];
  organizationStatus: OrganizationStatus;
  playStatus: PlayStatus;
  rating: number | null;
  version: string | null;
  language: string | null;
  notes: string | null;
}

export interface LibraryResponse {
  items: InventoryItem[];
}
