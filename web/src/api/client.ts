import type {
  ClassificationInput,
  CodeDef,
  CommonTag,
  ConfirmResponse,
  KeywordRule,
  InboxStatus,
  LibraryResponse,
  MoveBatch,
  MovePreview,
  OtherFilesReport,
  PlannedMove,
  RefetchStatus,
  Rules,
  SearchResponse,
  SourceName,
  Suggestion,
  Vocabulary,
  WorkTypeRule,
} from "./types";

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

export interface DeleteWithFileResult {
  fileDeleted: boolean;
  fileMissing: boolean;
}

export async function deleteItemAndFile(
  itemId: string,
): Promise<DeleteWithFileResult> {
  const r = await fetch(`${API_BASE}/api/metadata/delete-item-and-file`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ inventoryItemId: itemId }),
  });
  if (!r.ok) throw new Error(`Delete failed: ${r.status}`);
  return r.json();
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

/** Throw with the server's `{ error }` message when there is one. */
async function ensureOk(r: Response, what: string): Promise<Response> {
  if (r.ok) return r;
  let message = `${what} failed: ${r.status}`;
  try {
    const body: { error?: string } = await r.json();
    if (body.error) message = body.error;
  } catch {
    // not JSON; keep the status message
  }
  throw new Error(message);
}

function postJson(path: string, body: unknown): Promise<Response> {
  return fetch(`${API_BASE}${path}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
}

export async function fetchVocabulary(): Promise<Vocabulary> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/vocabulary`), "Vocabulary");
  return r.json();
}

export async function upsertCode(code: {
  code: string;
  label: string;
  categoryId: string;
  family?: string;
  acceptsPrefix?: boolean;
  aliases?: string[];
}): Promise<CodeDef> {
  const r = await ensureOk(await postJson("/api/vocabulary/codes", code), "Save code");
  return r.json();
}

export async function deleteCode(code: string): Promise<void> {
  await ensureOk(
    await fetch(`${API_BASE}/api/vocabulary/codes/${encodeURIComponent(code)}`, {
      method: "DELETE",
    }),
    "Delete code",
  );
}

export async function confirmClassification(items: ClassificationInput[]): Promise<number> {
  const r = await ensureOk(
    await postJson("/api/classification/confirm", { items }),
    "Confirm classification",
  );
  const data: { updated: number } = await r.json();
  return data.updated;
}

export async function fetchRefetchStatus(): Promise<RefetchStatus> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/sources/refetch`), "Re-fetch status");
  return r.json();
}

export async function startRefetch(): Promise<RefetchStatus> {
  const r = await ensureOk(
    await fetch(`${API_BASE}/api/sources/refetch`, { method: "POST" }),
    "Start re-fetch",
  );
  return r.json();
}

export async function fetchSuggestions(): Promise<Suggestion[]> {
  const r = await ensureOk(
    await fetch(`${API_BASE}/api/classification/suggestions`),
    "Suggestions",
  );
  const data: { suggestions: Suggestion[] } = await r.json();
  return data.suggestions;
}

export async function fetchRules(): Promise<Rules> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/classification/rules`), "Rules");
  return r.json();
}

export async function saveWorkTypeRule(rule: WorkTypeRule): Promise<WorkTypeRule> {
  const r = await ensureOk(
    await postJson("/api/classification/rules/work-type", rule),
    "Save rule",
  );
  return r.json();
}

export async function deleteWorkTypeRule(source: SourceName, workType: string): Promise<void> {
  await ensureOk(
    await fetch(
      `${API_BASE}/api/classification/rules/work-type/${source}/${encodeURIComponent(workType)}`,
      { method: "DELETE" },
    ),
    "Delete rule",
  );
}

export async function saveKeywordRule(
  rule: Omit<KeywordRule, "id"> & { id?: string },
): Promise<KeywordRule> {
  const r = await ensureOk(await postJson("/api/classification/rules/keyword", rule), "Save rule");
  return r.json();
}

export async function deleteKeywordRule(id: string): Promise<void> {
  await ensureOk(
    await fetch(`${API_BASE}/api/classification/rules/keyword/${id}`, { method: "DELETE" }),
    "Delete rule",
  );
}

export async function fetchCommonTags(): Promise<CommonTag[]> {
  const r = await ensureOk(
    await fetch(`${API_BASE}/api/classification/common-tags`),
    "Common tags",
  );
  const data: { tags: CommonTag[] } = await r.json();
  return data.tags;
}

export async function fetchMovePlan(): Promise<PlannedMove[]> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/organize/plan`), "Move plan");
  const data: { moves: PlannedMove[] } = await r.json();
  return data.moves;
}

export async function previewMoves(inventoryItemIds: string[]): Promise<MovePreview> {
  const r = await ensureOk(
    await postJson("/api/organize/preview", { inventoryItemIds }),
    "Preview",
  );
  return r.json();
}

export async function executeMoves(inventoryItemIds: string[]): Promise<MoveBatch> {
  const r = await ensureOk(
    await postJson("/api/organize/execute", { inventoryItemIds }),
    "Move",
  );
  return r.json();
}

export async function fetchBatches(): Promise<MoveBatch[]> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/organize/batches`), "Batches");
  const data: { batches: MoveBatch[] } = await r.json();
  return data.batches;
}

export async function fetchBatch(id: string): Promise<MoveBatch> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/organize/batches/${id}`), "Batch");
  return r.json();
}

export async function undoBatch(id: string): Promise<MoveBatch> {
  const r = await ensureOk(
    await fetch(`${API_BASE}/api/organize/batches/${id}/undo`, { method: "POST" }),
    "Undo",
  );
  return r.json();
}

export async function fetchInbox(): Promise<InboxStatus> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/organize/inbox`), "Inbox");
  return r.json();
}

export async function createInbox(): Promise<InboxStatus> {
  const r = await ensureOk(
    await fetch(`${API_BASE}/api/organize/inbox`, { method: "POST" }),
    "Create inbox",
  );
  return r.json();
}

export async function fetchOtherFiles(): Promise<OtherFilesReport> {
  const r = await ensureOk(await fetch(`${API_BASE}/api/organize/other-files`), "Other files");
  return r.json();
}
