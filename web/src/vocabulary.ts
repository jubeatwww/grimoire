// Client-side mirror of grimoire-app's vocabulary rules, used for search,
// filtering and input hints. The server stays the authority: it re-normalises
// and validates everything on save.
import type { CategoryDef, CodeDef, SourceName, Vocabulary } from "./api/types";

export const VARIANT_PREFIXES = ["T-", "X-"] as const;

const PREFIX_LABELS: Record<string, string> = { "T-": "觸摸", "X-": "局部/斷面" };

export const SOURCE_LABELS: Record<SourceName, string> = {
  dlsite: "DLsite",
  vndb: "VNDB",
  steam: "Steam",
};

const SUPERSCRIPTS = "⁰¹²³⁴⁵⁶⁷⁸⁹";

/** Uppercase, trim, and turn trailing ASCII digits into superscripts. */
export function canonicalForm(input: string): string {
  const upper = input.trim().toUpperCase();
  const match = upper.match(/^(.*\D)(\d+)$/);
  if (!match) return upper;
  const [, head, digits] = match;
  return head + [...digits].map((d) => SUPERSCRIPTS[Number(d)]).join("");
}

export interface ResolvedCode {
  prefix: string;
  base: CodeDef;
  canonical: string;
}

export function resolveCode(vocab: Vocabulary, input: string): ResolvedCode | null {
  const key = canonicalForm(input);
  if (!key) return null;
  const exact = vocab.codes.find(
    (c) => c.code.toUpperCase() === key || c.aliases.some((a) => a.toUpperCase() === key),
  );
  if (exact) return { prefix: "", base: exact, canonical: exact.code };
  for (const prefix of VARIANT_PREFIXES) {
    if (!key.startsWith(prefix)) continue;
    const rest = key.slice(prefix.length);
    const base = vocab.codes.find((c) => c.acceptsPrefix && c.code.toUpperCase() === rest);
    return base ? { prefix, base, canonical: prefix + base.code } : null;
  }
  return null;
}

export function categoryById(vocab: Vocabulary | undefined, id: string | null): CategoryDef | null {
  if (!vocab || !id) return null;
  return vocab.categories.find((c) => c.id === id) ?? null;
}

/** "04 角色扮演(RPG)" for a category id, or the id itself when unknown. */
export function categoryName(vocab: Vocabulary | undefined, id: string | null): string {
  if (!id) return "Unsorted";
  return categoryById(vocab, id)?.folderName ?? id;
}

/** Label shown on hover for a stored code, including its variant prefix. */
export function codeTitle(vocab: Vocabulary | undefined, code: string): string {
  if (!vocab) return code;
  const r = resolveCode(vocab, code);
  if (!r) return code;
  return variantLabel(r.prefix, r.base);
}

function variantLabel(prefix: string, base: CodeDef): string {
  return prefix ? `${PREFIX_LABELS[prefix]} · ${base.label}` : base.label;
}

export interface CodeOption {
  code: string;
  label: string;
  categoryId: string;
}

/**
 * Every code a picker can offer, in folder order: each base code followed by
 * its T- and X- forms when it accepts them.
 */
export function codeOptions(vocab: Vocabulary | undefined): CodeOption[] {
  if (!vocab) return [];
  const categoryRank = new Map(vocab.categories.map((c) => [c.id, c.sortOrder]));
  const rank = (c: CodeDef) => categoryRank.get(c.categoryId) ?? Number.MAX_SAFE_INTEGER;
  return [...vocab.codes]
    .sort((a, b) => rank(a) - rank(b) || a.sortOrder - b.sortOrder)
    .flatMap((base) =>
      ["", ...(base.acceptsPrefix ? VARIANT_PREFIXES : [])].map((prefix) => ({
        code: prefix + base.code,
        label: variantLabel(prefix, base),
        categoryId: base.categoryId,
      })),
    );
}

/**
 * True when a search query names one of the item's codes. A family query
 * (HAG) matches every variant (HAG², T-HAG), and a base code query (HAG²)
 * also matches its prefixed forms (T-HAG²).
 */
export function codesMatchQuery(vocab: Vocabulary | undefined, codes: string[], query: string): boolean {
  const key = canonicalForm(query);
  if (!key) return false;
  return codes.some((code) => {
    const r = vocab ? resolveCode(vocab, code) : null;
    if (!r) return code.toUpperCase() === key;
    return r.canonical === key || r.base.code === key || r.base.family === key;
  });
}
