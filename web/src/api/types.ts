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
  displayTitle: string | null;
  coverImageUrl: string | null;
}

export interface LibraryResponse {
  items: InventoryItem[];
}

export interface MetadataCandidate {
  id: string;
  sourceName: string;
  sourceWorkId: string;
  sourceUrl: string;
  rank: number;
  title: string;
  circle: string | null;
  coverUrl: string | null;
  workType: string | null;
  introShort: string | null;
}

export interface SearchResponse {
  candidates: MetadataCandidate[];
}

export interface ConfirmResponse {
  gameWorkId: string;
}
