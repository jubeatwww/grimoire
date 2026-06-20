import { useEffect, useState } from "react";
import {
  confirmCandidate,
  downloadUrl,
  linkInventoryItem,
  refreshMetadata,
  searchMetadata,
} from "../api/client";
import type { InventoryItem, MetadataCandidate } from "../api/types";
import { useImagePreview } from "./useImagePreview";

interface DetailPanelProps {
  item: InventoryItem | null;
  autoSearchToken?: number;
  onMetadataConfirmed?: () => void;
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let v = n / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(v < 10 ? 2 : 1)} ${units[i]}`;
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

export function DetailPanel({ item, autoSearchToken, onMetadataConfirmed }: DetailPanelProps) {
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
      if (result.candidates.length === 0) {
        setError("No results found");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Search failed");
    } finally {
      setSearching(false);
    }
  };

  useEffect(() => {
    if (item) {
      setQuery(cleanQuery(item.fileName));
      setCandidates([]);
      setError(null);
      setLinkInput("");
    }
  }, [item?.id]);

  const runRefresh = async (itemId: string) => {
    setError(null);
    setSearching(true);
    try {
      await refreshMetadata(itemId);
      onMetadataConfirmed?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : "Refresh failed");
    } finally {
      setSearching(false);
    }
  };

  useEffect(() => {
    if (!autoSearchToken || autoSearchToken === 0 || !item) return;
    // Branch by item state: pending → search candidates;
    // confirmed-without-description → refresh detail from DLsite.
    if (item.organizationStatus === "confirmed" && !item.description) {
      void runRefresh(item.id);
    } else {
      const q = cleanQuery(item.fileName);
      setQuery(q);
      void runSearch(q);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoSearchToken]);

  if (!item) {
    return <div className="empty-detail">Select a game</div>;
  }

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
      onMetadataConfirmed?.();
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
      <div className="detail-title-row">
        <h2>{item.displayTitle ?? item.fileName}</h2>
        <a
          className="icon-button"
          href={downloadUrl(item.id)}
          title="Download"
          aria-label="Download"
        >
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
            <path d="M12 3v12" />
            <path d="m6 11 6 6 6-6" />
            <path d="M5 21h14" />
          </svg>
        </a>
      </div>
      {item.displayTitle && <p className="detail-filename">{item.fileName}</p>}
      {item.circle && <p className="detail-circle">{item.circle}</p>}
      <div className="detail-status-row">
        <span className={`status-pill status-${item.organizationStatus}`}>
          {item.organizationStatus}
        </span>
        {(item.workTypeLabel || item.workType) && (
          <span
            className="work-type-pill"
            title={item.workType ?? undefined}
          >
            {item.workTypeLabel ?? item.workType}
          </span>
        )}
        <span>{item.primaryCategory ?? "Unsorted"}</span>
        {item.fileType && <span>· {item.fileType}</span>}
        {item.fileSizeBytes != null && (
          <span>· {formatBytes(item.fileSizeBytes)}</span>
        )}
      </div>
      {item.rateAverage != null && item.rateCount != null && item.rateCount > 0 && (
        <p className="detail-rating">
          <span className="rating-stars">{"★".repeat(Math.round(item.rateAverage))}</span>
          <span>{item.rateAverage.toFixed(2)}</span>
          <span className="rating-count">({item.rateCount.toLocaleString()})</span>
          {item.dlCount != null && (
            <span className="rating-dlcount">· {item.dlCount.toLocaleString()} DL</span>
          )}
        </p>
      )}
      {item.sourceTags && item.sourceTags.length > 0 && (
        <div className="detail-tags">
          {item.sourceTags.map((t) => (
            <span key={t} className="detail-tag">{t}</span>
          ))}
        </div>
      )}
      {item.description && (
        <p className="detail-description">{item.description}</p>
      )}
      {item.previewImageUrls && item.previewImageUrls.length > 0 && (
        <div className="detail-previews">
          {item.previewImageUrls.map((url) => (
            <a
              key={url}
              href={url}
              target="_blank"
              rel="noreferrer"
              className="detail-preview"
            >
              <img src={url} alt="" loading="lazy" />
            </a>
          ))}
        </div>
      )}
      <dl>
        {item.releaseDate && (
          <>
            <dt>Release</dt>
            <dd>{item.releaseDate}</dd>
          </>
        )}
        {item.series && (
          <>
            <dt>Series</dt>
            <dd>{item.series}</dd>
          </>
        )}
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
      <div className="detail-link">
        <input
          type="text"
          value={linkInput}
          onChange={(e) => setLinkInput(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleLink()}
          placeholder="Direct link · paste DLsite URL or RJ/VJ/BJ code"
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
          <h3>Candidates</h3>
          <ul className="candidate-list">
            {candidates.map((c) => {
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
                        title="Open on DLsite"
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
        </div>
      )}
      {preview}
    </div>
  );
}
