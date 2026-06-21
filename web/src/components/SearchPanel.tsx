import { useEffect, useState } from "react";
import {
  confirmCandidate,
  linkInventoryItem,
  refreshMetadata,
  searchMetadata,
} from "../api/client";
import type { InventoryItem, MetadataCandidate } from "../api/types";
import { useImagePreview } from "./useImagePreview";

interface SearchPanelProps {
  item: InventoryItem;
  autoSearchToken?: number;
  onChanged?: () => void;
}

function cleanQuery(filename: string): string {
  return filename
    .replace(/\.(zip|rar|7z|exe|iso)$/i, "")
    .replace(/[vV]?\d+\.\d+[\d.]*/g, "")
    .replace(/\+\d+/g, "")
    .replace(/\[.*?\]/g, "")
    .replace(/\(.*?\)/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

const SOURCE_LABEL: Record<string, string> = {
  dlsite: "DLsite",
  vndb: "VNDB",
};
const SOURCE_ORDER: Record<string, number> = { dlsite: 0, vndb: 1 };

function groupBySource(
  candidates: MetadataCandidate[],
): [string, MetadataCandidate[]][] {
  const map = new Map<string, MetadataCandidate[]>();
  for (const c of candidates) {
    const arr = map.get(c.sourceName) ?? [];
    arr.push(c);
    map.set(c.sourceName, arr);
  }
  return [...map.entries()].sort(
    (a, b) => (SOURCE_ORDER[a[0]] ?? 99) - (SOURCE_ORDER[b[0]] ?? 99),
  );
}

export function SearchPanel({ item, autoSearchToken, onChanged }: SearchPanelProps) {
  const [candidates, setCandidates] = useState<MetadataCandidate[]>([]);
  const [searching, setSearching] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [linkInput, setLinkInput] = useState("");
  const [linking, setLinking] = useState(false);
  const { hoverProps, preview, clear: clearPreview } = useImagePreview();
  const [coverErrored, setCoverErrored] = useState<Set<string>>(new Set());

  const markErrored = (id: string) =>
    setCoverErrored((prev) => (prev.has(id) ? prev : new Set(prev).add(id)));

  const runSearch = async (q: string) => {
    if (!q.trim()) return;
    setSearching(true);
    setError(null);
    setCandidates([]);
    clearPreview();
    try {
      const result = await searchMetadata(q.trim());
      setCandidates(result.candidates);
      if (result.candidates.length === 0) setError("No results found");
    } catch (e) {
      setError(e instanceof Error ? e.message : "Search failed");
    } finally {
      setSearching(false);
    }
  };

  const runRefresh = async (source: "dlsite" | "vndb") => {
    setError(null);
    setSearching(true);
    try {
      await refreshMetadata(item.id, source);
      onChanged?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Refresh failed");
    } finally {
      setSearching(false);
    }
  };

  useEffect(() => {
    setQuery(cleanQuery(item.fileName));
    setCandidates([]);
    setError(null);
    setLinkInput("");
  }, [item.id]);

  useEffect(() => {
    if (!autoSearchToken || autoSearchToken === 0) return;
    // confirmed + already enriched → nothing to auto-do.
    // confirmed + not yet enriched → refresh the source it has.
    // otherwise → unified search.
    if (item.organizationStatus === "confirmed" && !item.enrichedAt) {
      const src = item.vndbId ? "vndb" : item.dlsiteWorkId ? "dlsite" : null;
      if (src) void runRefresh(src);
    } else if (
      item.organizationStatus !== "confirmed" ||
      !item.description
    ) {
      const q = cleanQuery(item.fileName);
      setQuery(q);
      void runSearch(q);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoSearchToken]);

  const handleSearch = () => runSearch(query);

  const handleLink = async () => {
    const v = linkInput.trim();
    if (!v) return;
    setLinking(true);
    setError(null);
    try {
      await linkInventoryItem(item.id, v);
      setLinkInput("");
      setCandidates([]);
      clearPreview();
      onChanged?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Link failed");
    } finally {
      setLinking(false);
    }
  };

  const handleConfirm = async (candidate: MetadataCandidate) => {
    setConfirming(candidate.id);
    setError(null);
    try {
      await confirmCandidate(candidate.id, item.id);
      setCandidates([]);
      clearPreview();
      onChanged?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Confirm failed");
    } finally {
      setConfirming(null);
    }
  };

  return (
    <div className="search-panel">
      <div className="detail-search">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          placeholder="Search DLsite + VNDB..."
        />
        <button onClick={handleSearch} disabled={searching || !query.trim()}>
          {searching ? "..." : "Search"}
        </button>
      </div>
      <div className="detail-link">
        <input
          type="text"
          value={linkInput}
          onChange={(e) => setLinkInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleLink()}
          placeholder="Direct link · RJ/VJ/BJ code, vN code, or full URL"
        />
        <button
          onClick={handleLink}
          disabled={linking || !linkInput.trim()}
          className="detail-link-button"
        >
          {linking ? "..." : "Link"}
        </button>
      </div>
      {error && <p className="error-message">{error}</p>}
      {candidates.length > 0 && (
        <div className="candidates">
          {groupBySource(candidates).map(([source, group]) => (
            <section key={source} className="candidate-group">
              <h3>
                <span className={`candidate-source-tag candidate-source-${source}`}>
                  {SOURCE_LABEL[source] ?? source}
                </span>
                <span className="candidate-group-count">· {group.length}</span>
              </h3>
              <ul className="candidate-list">
                {group.map((c) => {
                  const coverOk = c.coverUrl && !coverErrored.has(c.id);
                  return (
                    <li key={c.id} className="candidate-item" {...hoverProps(coverOk ? c.coverUrl : null)}>
                      {coverOk ? (
                        <img
                          src={c.coverUrl!}
                          alt=""
                          className="candidate-cover"
                          onError={() => markErrored(c.id)}
                        />
                      ) : (
                        <div className="candidate-cover candidate-cover-fallback" aria-hidden />
                      )}
                      <div className="candidate-info">
                        <strong title={c.title}>{c.title}</strong>
                        <div className="candidate-meta">
                          <a
                            className="candidate-link"
                            href={c.sourceUrl}
                            target="_blank"
                            rel="noreferrer"
                            title={`Open on ${SOURCE_LABEL[source] ?? source}`}
                          >
                            {c.sourceWorkId} <span aria-hidden>↗</span>
                          </a>
                          {c.workType && <span className="candidate-type">{c.workType}</span>}
                          {c.circle && <span className="candidate-circle">{c.circle}</span>}
                        </div>
                        {c.introShort && <p className="candidate-intro">{c.introShort}</p>}
                        <button
                          className="candidate-confirm"
                          onClick={() => handleConfirm(c)}
                          disabled={confirming === c.id}
                        >
                          {confirming === c.id ? "…" : "Confirm"}
                        </button>
                      </div>
                    </li>
                  );
                })}
              </ul>
            </section>
          ))}
        </div>
      )}
      {preview}
    </div>
  );
}
