import { useEffect, useState } from "react";
import { confirmCandidate, downloadUrl, searchMetadata } from "../api/client";
import type { InventoryItem, MetadataCandidate } from "../api/types";

interface DetailPanelProps {
  item: InventoryItem | null;
  onMetadataConfirmed?: () => void;
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

export function DetailPanel({ item, onMetadataConfirmed }: DetailPanelProps) {
  const [candidates, setCandidates] = useState<MetadataCandidate[]>([]);
  const [searching, setSearching] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [query, setQuery] = useState("");

  useEffect(() => {
    if (item) {
      setQuery(cleanQuery(item.fileName));
      setCandidates([]);
      setError(null);
    }
  }, [item?.id]);

  if (!item) {
    return <div className="empty-detail">Select a game</div>;
  }

  const handleSearch = async () => {
    if (!query.trim()) return;
    setSearching(true);
    setError(null);
    setCandidates([]);
    try {
      const result = await searchMetadata(query.trim());
      setCandidates(result.candidates);
      if (result.candidates.length === 0) {
        setError("No results found");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Search failed");
    } finally {
      setSearching(false);
    }
  };

  const handleConfirm = async (candidate: MetadataCandidate) => {
    setConfirming(candidate.id);
    setError(null);
    try {
      await confirmCandidate(candidate.id, item.id);
      setCandidates([]);
      onMetadataConfirmed?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Confirm failed");
    } finally {
      setConfirming(null);
    }
  };

  return (
    <div className="detail">
      {item.coverImageUrl ? (
        <img className="large-cover large-cover-image" src={item.coverImageUrl} alt="" />
      ) : (
        <div className="large-cover" />
      )}
      <h2>{item.displayTitle ?? item.fileName}</h2>
      {item.displayTitle && <p className="detail-filename">{item.fileName}</p>}
      <p>{item.primaryCategory ?? "Unsorted"} · {item.organizationStatus}</p>
      <dl>
        <dt>Legacy location</dt>
        <dd>{item.legacyLocation ?? "none"}</dd>
        <dt>Version</dt>
        <dd>{item.version ?? "unknown"}</dd>
        <dt>Language</dt>
        <dd>{item.language ?? "unknown"}</dd>
      </dl>
      <div className="detail-search">
        <input
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleSearch()}
          placeholder="Search DLsite..."
        />
        <button onClick={handleSearch} disabled={searching || !query.trim()}>
          {searching ? "..." : "Search"}
        </button>
      </div>
      <div className="detail-actions">
        <a className="button" href={downloadUrl(item.id)}>Download</a>
      </div>
      {error && <p className="error-message">{error}</p>}
      {candidates.length > 0 && (
        <div className="candidates">
          <h3>Candidates</h3>
          <ul className="candidate-list">
            {candidates.map((c) => (
              <li key={c.id} className="candidate-item">
                {c.coverUrl && <img src={c.coverUrl} alt="" className="candidate-cover" />}
                <div className="candidate-info">
                  <strong>{c.title}</strong>
                  <small>{c.circle ?? "Unknown circle"} · {c.sourceWorkId}</small>
                </div>
                <button
                  className="primary"
                  onClick={() => handleConfirm(c)}
                  disabled={confirming === c.id}
                >
                  {confirming === c.id ? "..." : "Confirm"}
                </button>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
