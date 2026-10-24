import type { SourceTag, Vocabulary } from "../api/types";
import { categoryName, codeTitle, SOURCE_LABELS } from "../vocabulary";

/** Two-part pill: the tag on the left, the site it came from on the right. */
export function SourceTagPill({ tag }: { tag: SourceTag }) {
  return (
    <span className={`tag-pill tag-pill-${tag.source}`}>
      <span className="tag-pill-label">{tag.label}</span>
      <span className="tag-pill-source">{SOURCE_LABELS[tag.source] ?? tag.source}</span>
    </span>
  );
}

interface CodePillProps {
  code: string;
  vocab: Vocabulary | undefined;
  onRemove?: () => void;
}

/** A forum genre code. Hover shows its Chinese label. */
export function CodePill({ code, vocab, onRemove }: CodePillProps) {
  return (
    <span className="code-pill" title={codeTitle(vocab, code)}>
      {code}
      {onRemove && (
        <button
          type="button"
          className="code-pill-remove"
          onClick={onRemove}
          aria-label={`Remove ${code}`}
        >
          ×
        </button>
      )}
    </span>
  );
}

interface CategoryPillProps {
  categoryId: string | null;
  confirmed: boolean;
  vocab: Vocabulary | undefined;
}

/** The primary category, which decides the NAS folder. */
export function CategoryPill({ categoryId, confirmed, vocab }: CategoryPillProps) {
  if (!categoryId) return null;
  return (
    <span
      className={`category-pill ${confirmed ? "" : "category-pill-unconfirmed"}`}
      title={confirmed ? "Primary category" : "Primary category, not confirmed yet"}
    >
      ★ {categoryName(vocab, categoryId)}
    </span>
  );
}
