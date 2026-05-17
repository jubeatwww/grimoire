import { useState } from "react";
import { confirmCandidate, downloadUrl, searchMetadata } from "../api/client";
import type { InventoryItem, MetadataCandidate } from "../api/types";

interface DetailPanelProps {
  item: InventoryItem | null;
  onMetadataConfirmed?: () => void;
}

export function DetailPanel({ item, onMetadataConfirmed }: DetailPanelProps) {
  const [candidates, setCandidates] = useState<MetadataCandidate[]>([]);
  const [searching, setSearching] = useState(false);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!item) {
    return <div className="empty-detail">Select a game</div>;
  }

  const handleSearch = async () => {
    setSearching(true);
    setError(null);
    setCandidates([]);
    try {
      const result = await searchMetadata(item.fileName);
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
      <div className="large-cover" />
      <h2>{item.fileName}</h2>
      <p>{item.primaryCategory ?? "Unsorted"} · {item.organizationStatus}</p>
      <dl>
        <dt>Legacy location</dt>
        <dd>{item.legacyLocation ?? "none"}</dd>
        <dt>Version</dt>
        <dd>{item.version ?? "unknown"}</dd>
        <dt>Language</dt>
        <dd>{item.language ?? "unknown"}</dd>
      </dl>
      <div className="detail-actions">
        <button onClick={handleSearch} disabled={searching}>
          {searching ? "Searching..." : "Search DLsite"}
        </button>
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
