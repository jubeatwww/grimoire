import { useMemo, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  createInbox,
  executeMoves,
  fetchBatch,
  fetchBatches,
  fetchInbox,
  fetchMovePlan,
  previewMoves,
  undoBatch,
} from "../api/client";
import { invalidateLibrary, useVocabulary } from "../api/queries";
import type { MoveBatch, MovePreview, PlannedMove } from "../api/types";
import { categoryName } from "../vocabulary";
import { useConfirm } from "./ConfirmDialog";

const PLAN_KEY = ["move-plan"] as const;
const BATCHES_KEY = ["move-batches"] as const;
const INBOX_KEY = ["inbox"] as const;

function errorText(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/**
 * Moves confirmed works into the new NAS layout: pick files, preview, execute,
 * and undo a whole batch from the history.
 */
export function MoveView() {
  const vocab = useVocabulary();
  const queryClient = useQueryClient();
  const planQuery = useQuery({ queryKey: PLAN_KEY, queryFn: fetchMovePlan });
  const plan = planQuery.data ?? [];
  const [checked, setChecked] = useState<Set<string>>(new Set());
  const [preview, setPreview] = useState<MovePreview | null>(null);
  const [lastBatch, setLastBatch] = useState<MoveBatch | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const groups = useMemo(() => {
    const byCategory = new Map<string, PlannedMove[]>();
    for (const m of plan) {
      const list = byCategory.get(m.categoryId) ?? [];
      list.push(m);
      byCategory.set(m.categoryId, list);
    }
    const order = (id: string) => vocab?.categories.find((c) => c.id === id)?.sortOrder ?? 999;
    return [...byCategory.entries()].sort((a, b) => order(a[0]) - order(b[0]));
  }, [plan, vocab]);

  const movable = (m: PlannedMove) => !m.conflict;
  const toggle = (ids: string[], on: boolean) =>
    setChecked((prev) => {
      const next = new Set(prev);
      for (const id of ids) {
        if (on) next.add(id);
        else next.delete(id);
      }
      return next;
    });

  const selectedIds = plan.filter((m) => checked.has(m.inventoryItemId)).map((m) => m.inventoryItemId);

  const refresh = () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: PLAN_KEY }),
      queryClient.invalidateQueries({ queryKey: BATCHES_KEY }),
      invalidateLibrary(queryClient),
    ]);

  const runPreview = async () => {
    setBusy(true);
    setError(null);
    try {
      setPreview(await previewMoves(selectedIds));
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  const runExecute = async () => {
    if (!preview) return;
    setBusy(true);
    setError(null);
    try {
      const batch = await executeMoves(preview.moves.map((m) => m.inventoryItemId));
      setLastBatch(batch);
      setPreview(null);
      setChecked(new Set());
      await refresh();
    } catch (e) {
      setError(errorText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="move-view">
      <InboxBanner />
      <header className="classify-view-header">
        <div>
          <h2>Move files</h2>
          <p>
            {plan.length} file{plan.length === 1 ? "" : "s"} of confirmed works are not in their
            category folder yet. Moves never overwrite, and every batch can be undone.
          </p>
        </div>
        <div className="classify-view-actions">
          <button
            type="button"
            onClick={() => void queryClient.invalidateQueries({ queryKey: PLAN_KEY })}
            disabled={planQuery.isFetching}
          >
            {planQuery.isFetching ? "Refreshing…" : "Refresh plan"}
          </button>
          <button
            type="button"
            className="classify-save"
            onClick={runPreview}
            disabled={busy || selectedIds.length === 0}
          >
            Preview {selectedIds.length}
          </button>
        </div>
      </header>
      {error && <p className="settings-error">{error}</p>}
      {lastBatch && <BatchResult batch={lastBatch} onClose={() => setLastBatch(null)} />}

      {preview && (
        <section className="move-preview">
          <h3>Preview</h3>
          <MoveList moves={preview.moves} />
          {preview.removableDirs.length > 0 && (
            <>
              <h4>Folders that will be removed once empty</h4>
              <ul className="move-dirs">
                {preview.removableDirs.map((d) => (
                  <li key={d}>
                    <code>{d}</code>
                  </li>
                ))}
              </ul>
            </>
          )}
          <div className="move-preview-actions">
            <button type="button" onClick={() => setPreview(null)} disabled={busy}>
              Cancel
            </button>
            <button
              type="button"
              className="classify-save"
              onClick={runExecute}
              disabled={busy || preview.moves.every((m) => m.conflict)}
            >
              {busy
                ? "Moving…"
                : `Move ${preview.moves.filter((m) => !m.conflict).length} file(s)`}
            </button>
          </div>
        </section>
      )}

      {planQuery.isLoading && <p className="empty-state">Checking the library…</p>}
      {!planQuery.isLoading && plan.length === 0 && (
        <p className="empty-state">Every confirmed work is already in place.</p>
      )}
      {groups.map(([categoryId, moves]) => {
        const ids = moves.filter(movable).map((m) => m.inventoryItemId);
        const allOn = ids.length > 0 && ids.every((id) => checked.has(id));
        return (
          <details key={categoryId} className="move-group" open={groups.length <= 3}>
            <summary>
              <input
                type="checkbox"
                checked={allOn}
                disabled={ids.length === 0}
                onChange={(e) => toggle(ids, e.target.checked)}
                onClick={(e) => e.stopPropagation()}
                aria-label={`Select all of ${categoryName(vocab, categoryId)}`}
              />
              <strong>{categoryName(vocab, categoryId)}</strong>
              <span className="chip-count">
                {moves.length} file{moves.length === 1 ? "" : "s"}
                {moves.length !== ids.length && `, ${moves.length - ids.length} blocked`}
              </span>
            </summary>
            <MoveList
              moves={moves}
              checked={checked}
              onToggle={(id, on) => toggle([id], on)}
            />
          </details>
        );
      })}

      <BatchHistory onUndone={refresh} />
    </div>
  );
}

function MoveList({
  moves,
  checked,
  onToggle,
}: {
  moves: PlannedMove[];
  checked?: Set<string>;
  onToggle?: (id: string, on: boolean) => void;
}) {
  return (
    <ul className="move-list">
      {moves.map((m) => (
        <li key={m.inventoryItemId} className={m.conflict ? "blocked" : ""}>
          {onToggle && (
            <input
              type="checkbox"
              checked={checked?.has(m.inventoryItemId) ?? false}
              disabled={!!m.conflict}
              onChange={(e) => onToggle(m.inventoryItemId, e.target.checked)}
              aria-label={`Select ${m.title}`}
            />
          )}
          <div className="move-paths">
            <span className="move-title">{m.title}</span>
            <code className="move-from">{m.fromPath}</code>
            <code className="move-to">→ {m.toPath}</code>
            {m.conflict && <span className="move-conflict">⚠ {m.conflict}</span>}
          </div>
        </li>
      ))}
    </ul>
  );
}

function BatchResult({ batch, onClose }: { batch: MoveBatch; onClose: () => void }) {
  const problems = (batch.ops ?? []).filter((o) => o.status !== "moved");
  return (
    <section className="move-result">
      <p>
        <strong>Batch finished.</strong> Moved {batch.moved}, skipped {batch.skipped}, failed{" "}
        {batch.failed}.
        {batch.removedDirs.length > 0 && ` Removed ${batch.removedDirs.length} empty folder(s).`}
      </p>
      {problems.length > 0 && (
        <ul className="move-list">
          {problems.map((o) => (
            <li key={o.seq} className="blocked">
              <div className="move-paths">
                <code className="move-from">{o.fromPath}</code>
                <span className="move-conflict">
                  {o.status}: {o.message}
                </span>
              </div>
            </li>
          ))}
        </ul>
      )}
      {batch.notes.map((n) => (
        <p key={n} className="settings-error">
          {n}
        </p>
      ))}
      <button type="button" onClick={onClose}>
        Dismiss
      </button>
    </section>
  );
}

function InboxBanner() {
  const queryClient = useQueryClient();
  const inbox = useQuery({ queryKey: INBOX_KEY, queryFn: fetchInbox }).data;
  const [error, setError] = useState<string | null>(null);
  if (!inbox || inbox.exists) return null;
  const create = async () => {
    setError(null);
    try {
      await createInbox();
      await queryClient.invalidateQueries({ queryKey: INBOX_KEY });
    } catch (e) {
      setError(errorText(e));
    }
  };
  return (
    <div className="move-inbox">
      <span>
        The inbox folder <code>{inbox.folder}</code> does not exist yet. New downloads go there.
      </span>
      <button type="button" onClick={create}>
        Create it
      </button>
      {error && <span className="settings-error">{error}</span>}
    </div>
  );
}

function BatchHistory({ onUndone }: { onUndone: () => Promise<unknown> }) {
  const confirm = useConfirm();
  const batches = useQuery({ queryKey: BATCHES_KEY, queryFn: fetchBatches }).data ?? [];
  const [open, setOpen] = useState<MoveBatch | null>(null);
  const [error, setError] = useState<string | null>(null);

  const showDetails = async (id: string) => {
    setError(null);
    try {
      setOpen(await fetchBatch(id));
    } catch (e) {
      setError(errorText(e));
    }
  };

  const undo = async (batch: MoveBatch) => {
    const ok = await confirm({
      title: "Undo this batch?",
      message: `Moves the ${batch.moved} file(s) of this batch back to where they were. Files whose original location is taken again stay where they are.`,
      confirmLabel: "Undo batch",
      danger: true,
    });
    if (!ok) return;
    setError(null);
    try {
      setOpen(await undoBatch(batch.id));
      await onUndone();
    } catch (e) {
      setError(errorText(e));
    }
  };

  if (batches.length === 0) return null;
  return (
    <section className="move-history">
      <h3>History</h3>
      {error && <p className="settings-error">{error}</p>}
      <table className="settings-table">
        <thead>
          <tr>
            <th>When</th>
            <th>Status</th>
            <th>Moved</th>
            <th>Skipped</th>
            <th>Failed</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {batches.map((b) => (
            <tr key={b.id}>
              <td>{new Date(b.createdAt).toLocaleString()}</td>
              <td>{b.status}</td>
              <td>{b.moved}</td>
              <td>{b.skipped}</td>
              <td>{b.failed}</td>
              <td className="settings-row-actions">
                <button type="button" onClick={() => void showDetails(b.id)}>
                  Details
                </button>
                {b.status === "done" && b.moved > 0 && (
                  <button type="button" onClick={() => void undo(b)}>
                    Undo
                  </button>
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {open && (
        <div className="move-batch-details">
          <header>
            <strong>Batch {new Date(open.createdAt).toLocaleString()}</strong> · {open.status}
            <button type="button" onClick={() => setOpen(null)}>
              Close
            </button>
          </header>
          <ul className="move-list">
            {(open.ops ?? []).map((o) => (
              <li key={o.seq} className={o.status === "moved" || o.status === "undone" ? "" : "blocked"}>
                <div className="move-paths">
                  <span className="move-title">{o.status}</span>
                  <code className="move-from">{o.fromPath}</code>
                  <code className="move-to">→ {o.toPath}</code>
                  {o.message && <span className="move-conflict">{o.message}</span>}
                </div>
              </li>
            ))}
          </ul>
          {open.removedDirs.length > 0 && (
            <p className="settings-hint">Removed folders: {open.removedDirs.join(", ")}</p>
          )}
          {open.notes.map((n) => (
            <p key={n} className="settings-hint">
              {n}
            </p>
          ))}
        </div>
      )}
    </section>
  );
}
