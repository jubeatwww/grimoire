import { useEffect, useMemo, useState } from "react";
import { confirmClassification } from "../api/client";
import { useSuggestions, useVocabulary } from "../api/queries";
import type { InventoryItem, Suggestion } from "../api/types";
import { categoryById, codeOptions, resolveCode } from "../vocabulary";
import { CategoryPill, CodePill } from "./Pills";

interface ClassificationEditorProps {
  item: InventoryItem;
  onChanged?: () => void;
}

function sameCodes(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((c, i) => c === b[i]);
}

/** What the editor starts with: the stored values, or the suggestion when
 *  nothing has been chosen for an unconfirmed work yet. */
function initialState(item: InventoryItem, suggestion: Suggestion | undefined) {
  const untouched =
    !item.categoryConfirmed && !item.primaryCategory && item.genreFacets.length === 0;
  if (untouched && suggestion) {
    return { category: suggestion.category ?? "", codes: suggestion.codes };
  }
  return { category: item.primaryCategory ?? "", codes: item.genreFacets };
}

/**
 * Primary category picker plus genre-code pills for the item's work. Saving
 * always confirms the classification. The category applies to every file of
 * the work, patches and DLC included.
 */
export function ClassificationEditor({ item, onChanged }: ClassificationEditorProps) {
  const vocab = useVocabulary();
  const suggestions = useSuggestions().byWork;
  const suggestion =
    item.gameWorkId && !item.categoryConfirmed ? suggestions.get(item.gameWorkId) : undefined;
  const [category, setCategory] = useState(() => initialState(item, suggestion).category);
  const [codes, setCodes] = useState<string[]>(() => initialState(item, suggestion).codes);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [saving, setSaving] = useState(false);

  const facetsKey = item.genreFacets.join("|");
  const suggestionKey = suggestion ? `${suggestion.category}|${suggestion.codes.join("|")}` : "";
  useEffect(() => {
    const init = initialState(item, suggestion);
    setCategory(init.category);
    setCodes(init.codes);
    setDraft("");
    setError(null);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [item.id, item.primaryCategory, facetsKey, suggestionKey]);

  const dirty =
    category !== (item.primaryCategory ?? "") || !sameCodes(codes, item.genreFacets);

  const options = useMemo(() => codeOptions(vocab), [vocab]);

  if (!item.gameWorkId) {
    return (
      <p className="classify-hint">
        Link a source or create a manual entry before choosing a category.
      </p>
    );
  }

  const addDraft = () => {
    if (!draft.trim() || !vocab) return;
    const resolved = resolveCode(vocab, draft);
    if (!resolved) {
      setError(`${draft.trim()} is not in the code list. Add it under Settings first.`);
      return;
    }
    // The first code also sets the primary category to the one it belongs to.
    if (codes.length === 0) setCategory(resolved.base.categoryId);
    setCodes((prev) => (prev.includes(resolved.canonical) ? prev : [...prev, resolved.canonical]));
    setDraft("");
    setError(null);
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await confirmClassification([
        {
          gameWorkId: item.gameWorkId!,
          primaryCategory: category || null,
          genreFacets: codes,
        },
      ]);
      onChanged?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setSaving(false);
    }
  };

  const canSave = !saving && (dirty || (!item.categoryConfirmed && category !== ""));

  const applySuggestion = () => {
    if (!suggestion) return;
    setCategory(suggestion.category ?? "");
    setCodes(suggestion.codes);
  };
  const showingSuggestion =
    !!suggestion &&
    category === (suggestion.category ?? "") &&
    sameCodes(codes, suggestion.codes);

  return (
    <div className="classify-editor">
      {suggestion && (
        <div className={`classify-suggestion ${suggestion.confident ? "confident" : ""}`}>
          <div className="classify-suggestion-head">
            <strong>{suggestion.confident ? "Suggested" : "Suggested, needs review"}</strong>
            <CategoryPill categoryId={suggestion.category} confirmed={false} vocab={vocab} />
            {suggestion.codes.map((code) => (
              <CodePill key={code} code={code} vocab={vocab} />
            ))}
            {!showingSuggestion && (
              <button type="button" className="classify-use" onClick={applySuggestion}>
                Use
              </button>
            )}
          </div>
          {suggestion.reasons.length > 0 && (
            <ul className="classify-reasons">
              {suggestion.reasons.map((r) => (
                <li key={r}>{r}</li>
              ))}
            </ul>
          )}
        </div>
      )}
      <div className="classify-row">
        <select
          className="classify-category"
          value={category}
          onChange={(e) => setCategory(e.target.value)}
          aria-label="Primary category"
        >
          <option value="">Unsorted</option>
          {vocab?.categories.map((c) => (
            <option key={c.id} value={c.id}>
              {c.folderName}
            </option>
          ))}
        </select>
        <span className={`classify-state ${item.categoryConfirmed ? "classify-state-ok" : ""}`}>
          {item.categoryConfirmed ? "✓ confirmed" : "not confirmed"}
        </span>
        <button type="button" className="classify-save" onClick={save} disabled={!canSave}>
          {saving ? "Saving…" : item.categoryConfirmed ? "Save" : "Confirm"}
        </button>
      </div>
      <div className="classify-codes">
        {codes.map((code) => (
          <CodePill
            key={code}
            code={code}
            vocab={vocab}
            onRemove={() => setCodes((prev) => prev.filter((c) => c !== code))}
          />
        ))}
        <input
          className="classify-code-input"
          list="genre-code-options"
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              addDraft();
            }
          }}
          onBlur={addDraft}
          placeholder="+ code"
          aria-label="Add genre code"
        />
        <datalist id="genre-code-options">
          {options.map((o) => (
            <option key={o.code} value={o.code}>
              {`${o.label} · ${categoryById(vocab, o.categoryId)?.label ?? o.categoryId}`}
            </option>
          ))}
        </datalist>
      </div>
      {error && <p className="classify-error">{error}</p>}
    </div>
  );
}
