import { useMemo, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { confirmClassification } from "../api/client";
import { invalidateLibrary, useSuggestions, useVocabulary } from "../api/queries";
import type { InventoryItem, Suggestion } from "../api/types";
import { CategoryPill, CodePill } from "./Pills";

interface ClassifyViewProps {
  items: InventoryItem[];
  selectedId: string | null;
  onSelect: (item: InventoryItem) => void;
  onReviewRest: () => void;
}

interface Row {
  suggestion: Suggestion;
  /** First present file of the work; the row shows its cover and title. */
  item: InventoryItem;
  fileCount: number;
}

/**
 * Batch confirmation of confident suggestions. Nothing here moves files; the
 * Move view picks confirmed works up afterwards.
 */
export function ClassifyView({ items, selectedId, onSelect, onReviewRest }: ClassifyViewProps) {
  const vocab = useVocabulary();
  const suggestions = useSuggestions().byWork;
  const queryClient = useQueryClient();
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<string | null>(null);

  const { rows, needsReview } = useMemo(() => {
    const byWork = new Map<string, InventoryItem[]>();
    for (const item of items) {
      if (!item.gameWorkId || item.missing) continue;
      const list = byWork.get(item.gameWorkId) ?? [];
      list.push(item);
      byWork.set(item.gameWorkId, list);
    }
    const rows: Row[] = [];
    let needsReview = 0;
    for (const suggestion of suggestions.values()) {
      const files = byWork.get(suggestion.gameWorkId);
      if (!files) continue;
      if (!suggestion.confident) {
        needsReview++;
        continue;
      }
      rows.push({ suggestion, item: files[0], fileCount: files.length });
    }
    rows.sort((a, b) => {
      const ca = vocab?.categories.find((c) => c.id === a.suggestion.category)?.sortOrder ?? 999;
      const cb = vocab?.categories.find((c) => c.id === b.suggestion.category)?.sortOrder ?? 999;
      return ca - cb || (a.item.displayTitle ?? "").localeCompare(b.item.displayTitle ?? "");
    });
    return { rows, needsReview };
  }, [items, suggestions, vocab]);

  const toggle = (id: string) =>
    setChecked((prev) => {
      const next = new Set(prev);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });
  const allChecked = rows.length > 0 && rows.every((r) => checked.has(r.suggestion.gameWorkId));
  const toggleAll = () =>
    setChecked(allChecked ? new Set() : new Set(rows.map((r) => r.suggestion.gameWorkId)));

  const selected = rows.filter((r) => checked.has(r.suggestion.gameWorkId));

  const confirmSelected = async () => {
    setBusy(true);
    setMessage(null);
    try {
      const updated = await confirmClassification(
        selected.map((r) => ({
          gameWorkId: r.suggestion.gameWorkId,
          primaryCategory: r.suggestion.category,
          genreFacets: r.suggestion.codes,
        })),
      );
      setChecked(new Set());
      setMessage(`Confirmed ${updated} work${updated === 1 ? "" : "s"}. Files have not moved yet.`);
      await invalidateLibrary(queryClient);
    } catch (e) {
      setMessage(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="classify-view">
      <header className="classify-view-header">
        <div>
          <h2>Ready to confirm</h2>
          <p>
            {rows.length} work{rows.length === 1 ? "" : "s"} with a confident suggestion.
            Confirming sets the category only. Files move later, from the Move view.
          </p>
          {needsReview > 0 && (
            <p>
              {needsReview} more need a decision.{" "}
              <button type="button" className="link-button" onClick={onReviewRest}>
                Review them in Organize →
              </button>
            </p>
          )}
        </div>
        <div className="classify-view-actions">
          <button type="button" onClick={toggleAll} disabled={rows.length === 0}>
            {allChecked ? "Select none" : "Select all"}
          </button>
          <button
            type="button"
            className="classify-save"
            onClick={confirmSelected}
            disabled={busy || selected.length === 0}
          >
            {busy ? "Confirming…" : `Confirm ${selected.length}`}
          </button>
        </div>
      </header>
      {message && <p className="classify-view-message">{message}</p>}
      {rows.length === 0 ? (
        <p className="empty-state">Nothing to batch-confirm right now.</p>
      ) : (
        <table className="classify-table">
          <tbody>
            {rows.map(({ suggestion, item, fileCount }) => (
              <tr
                key={suggestion.gameWorkId}
                className={selectedId === item.id ? "selected" : ""}
                onClick={() => onSelect(item)}
              >
                <td onClick={(e) => e.stopPropagation()}>
                  <input
                    type="checkbox"
                    checked={checked.has(suggestion.gameWorkId)}
                    onChange={() => toggle(suggestion.gameWorkId)}
                    aria-label={`Select ${item.displayTitle ?? item.fileName}`}
                  />
                </td>
                <td className="col-cover">
                  {item.coverImageUrl ? (
                    <img src={item.coverImageUrl} alt="" loading="lazy" />
                  ) : (
                    <div className="thumb-fallback" />
                  )}
                </td>
                <td className="col-title">
                  <strong>{item.displayTitle ?? item.fileName}</strong>
                  <small>
                    {item.circle ?? "no circle"} · {fileCount} file{fileCount === 1 ? "" : "s"}
                  </small>
                </td>
                <td>
                  <div className="classify-table-codes">
                    <CategoryPill categoryId={suggestion.category} confirmed={false} vocab={vocab} />
                    {suggestion.codes.map((code) => (
                      <CodePill key={code} code={code} vocab={vocab} />
                    ))}
                  </div>
                </td>
                <td className="classify-table-reasons">{suggestion.reasons.join(" · ")}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
