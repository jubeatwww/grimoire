export type OrganizationStatus =
  | "pending"
  | "matched"
  | "confirmed"
  | "ignored"
  | "no_match";
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
  circle: string | null;
  description: string | null;
  releaseDate: string | null;
  series: string | null;
  sourceTags: string[];
  previewImageUrls: string[];
  fileType: string | null;
  fileSizeBytes: number | null;
  dlCount: number | null;
  rateAverage: number | null;
  rateCount: number | null;
  priceJpy: number | null;
  workType: string | null;
  workTypeLabel: string | null;
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
