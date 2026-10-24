export type OrganizationStatus =
  | "pending"
  | "matched"
  | "confirmed"
  | "ignored"
  | "no_match";
export type PlayStatus = "not_played" | "want_to_play" | "playing" | "completed" | "dropped";

export type SourceName = "dlsite" | "vndb" | "steam";

export interface SourceTag {
  label: string;
  source: SourceName;
}

export interface InventoryItem {
  id: string;
  sourceId: string;
  /** Path relative to the library root. */
  path: string;
  fileName: string;
  legacyLocation: string | null;
  gameWorkId: string | null;
  /** Work-level category id, e.g. "RPG". Files inherit it from their work. */
  primaryCategory: string | null;
  categoryConfirmed: boolean;
  /** Work-level genre codes, e.g. ["ARPG", "T-HAG²"]. */
  genreFacets: string[];
  organizationStatus: OrganizationStatus;
  playStatus: PlayStatus;
  rating: number | null;
  version: string | null;
  language: string | null;
  notes: string | null;
  missing: boolean;
  displayTitle: string | null;
  coverImageUrl: string | null;
  circle: string | null;
  description: string | null;
  releaseDate: string | null;
  series: string | null;
  sourceTags: SourceTag[];
  previewImageUrls: string[];
  fileType: string | null;
  fileSizeBytes: number | null;
  dlCount: number | null;
  rateAverage: number | null;
  rateCount: number | null;
  priceJpy: number | null;
  workType: string | null;
  workTypeLabel: string | null;
  dlsiteWorkId: string | null;
  vndbId: string | null;
  steamAppId: string | null;
  enrichedAt: string | null;
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

export interface CategoryDef {
  id: string;
  label: string;
  folderName: string;
  sortOrder: number;
}

export interface CodeDef {
  code: string;
  label: string;
  categoryId: string;
  family: string;
  acceptsPrefix: boolean;
  aliases: string[];
  sortOrder: number;
}

export interface Vocabulary {
  categories: CategoryDef[];
  codes: CodeDef[];
}

export interface ClassificationInput {
  gameWorkId: string;
  primaryCategory: string | null;
  genreFacets: string[];
}

export interface RefetchStatus {
  running: boolean;
  total: number;
  done: number;
  failed: number;
  startedAt: string | null;
  finishedAt: string | null;
  lastError: string | null;
}

export interface Suggestion {
  gameWorkId: string;
  category: string | null;
  codes: string[];
  confident: boolean;
  reasons: string[];
}

export interface WorkTypeRule {
  source: SourceName;
  workType: string;
  /** null means "decide from the tags". */
  code: string | null;
  confident: boolean;
}

export interface KeywordRule {
  id: string;
  /** null applies to every source. */
  source: SourceName | null;
  keyword: string;
  code: string;
  contestsType: boolean;
}

export interface Rules {
  workTypeRules: WorkTypeRule[];
  keywordRules: KeywordRule[];
}

export interface CommonTag {
  label: string;
  source: SourceName;
  count: number;
}

export interface PlannedMove {
  inventoryItemId: string;
  gameWorkId: string;
  title: string;
  categoryId: string;
  fromPath: string;
  toPath: string;
  /** Why the file cannot move right now. */
  conflict: string | null;
}

export interface MovePreview {
  moves: PlannedMove[];
  removableDirs: string[];
}

export type MoveOpStatus =
  | "pending"
  | "undo_pending"
  | "moved"
  | "skipped"
  | "failed"
  | "undone"
  | "undo_skipped"
  | "undo_failed";

export interface MoveOp {
  seq: number;
  inventoryItemId: string | null;
  fromPath: string;
  toPath: string;
  status: MoveOpStatus;
  message: string | null;
}

export interface MoveBatch {
  id: string;
  status: "running" | "done" | "undoing" | "undone";
  moved: number;
  skipped: number;
  failed: number;
  removedDirs: string[];
  notes: string[];
  createdAt: string;
  finishedAt: string | null;
  undoneAt: string | null;
  ops?: MoveOp[];
}

export interface InboxStatus {
  folder: string;
  exists: boolean;
}

export interface OtherEntry {
  path: string;
  isDir: boolean;
  files: number;
  bytes: number;
}

export interface OtherFilesReport {
  entries: OtherEntry[];
  totalFiles: number;
  totalBytes: number;
}
