// Client-side mirror of grimoire-app's vocabulary rules, used for search,
// filtering and input hints. The server stays the authority: it re-normalises
// and validates everything on save.
import type { CategoryDef, CodeDef, SourceName, Vocabulary } from "./api/types";

export const VARIANT_PREFIXES = ["T-", "X-"] as const;
export type VariantPrefix = (typeof VARIANT_PREFIXES)[number];

/** Levels the code editor offers. The server accepts ² to ⁹. */
export const LEVEL_CHOICES = [2, 3, 4] as const;

const PREFIX_LABELS: Record<VariantPrefix, string> = { "T-": "觸摸", "X-": "局部/斷面" };

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

export function superscript(level: number): string {
  return [...String(level)].map((d) => SUPERSCRIPTS[Number(d)]).join("");
}

/** Split a single trailing ² to ⁹ off a code: "SSG³" → ["SSG", 3]. */
function splitLevel(key: string): [string, number] | null {
  const chars = [...key];
  const level = SUPERSCRIPTS.indexOf(chars.at(-1) ?? "");
  const head = chars.slice(0, -1);
  if (level < 2 || head.length === 0 || SUPERSCRIPTS.includes(head.at(-1)!)) return null;
  return [head.join(""), level];
}

function findExact(vocab: Vocabulary, key: string): CodeDef | undefined {
  return vocab.codes.find(
    (c) => c.code.toUpperCase() === key || c.aliases.some((a) => a.toUpperCase() === key),
  );
}

/** A code without its prefix: a row of its own, or a row plus a level. */
function resolveStem(vocab: Vocabulary, key: string): [CodeDef, number | null] | null {
  const exact = findExact(vocab, key);
  if (exact) return [exact, null];
  const split = splitLevel(key);
  const base = split && findExact(vocab, split[0]);
  return base ? [base, split[1]] : null;
}

export interface ResolvedCode {
  prefix: VariantPrefix | "";
  base: CodeDef;
  /** A level the base row does not spell out itself (SSG³), or null. */
  level: number | null;
  canonical: string;
}

/**
 * Resolve input or a stored code. A stored code is `[prefix]<code>[level]`:
 * any code takes a ² to ⁹ level, a named level row (HAG²) wins over the
 * generic reading, and T-/X- only combine with codes that accept them.
 */
export function resolveCode(vocab: Vocabulary, input: string): ResolvedCode | null {
  const key = canonicalForm(input);
  if (!key) return null;
  const build = (prefix: VariantPrefix | "", [base, level]: [CodeDef, number | null]) => ({
    prefix,
    base,
    level,
    canonical: prefix + base.code + (level ? superscript(level) : ""),
  });
  const stem = resolveStem(vocab, key);
  if (stem) return build("", stem);
  for (const prefix of VARIANT_PREFIXES) {
    if (!key.startsWith(prefix)) continue;
    const rest = resolveStem(vocab, key.slice(prefix.length));
    return rest && rest[0].acceptsPrefix ? build(prefix, rest) : null;
  }
  return null;
}

/** The parts of a code the editor toggles separately. */
export interface CodeParts {
  prefix: VariantPrefix | "";
  root: string;
  level: number | null;
}

/** Split a stored code into prefix, root and level. A named level row (HAG²)
 *  splits too, so its level changes like any other. */
export function codeParts(vocab: Vocabulary, code: string): CodeParts | null {
  const r = resolveCode(vocab, code);
  if (!r) return null;
  const stem = r.canonical.slice(r.prefix.length);
  const split = splitLevel(stem);
  if (split && findExact(vocab, split[0])) {
    return { prefix: r.prefix, root: split[0], level: split[1] };
  }
  return { prefix: r.prefix, root: stem, level: null };
}

/** The canonical code for a set of parts, or null when they do not combine. */
export function composeCode(vocab: Vocabulary, parts: CodeParts): string | null {
  const input = parts.prefix + parts.root + (parts.level ? superscript(parts.level) : "");
  return resolveCode(vocab, input)?.canonical ?? null;
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

/** Label shown on hover for a stored code, including its prefix and level. */
export function codeTitle(vocab: Vocabulary | undefined, code: string): string {
  if (!vocab) return code;
  const r = resolveCode(vocab, code);
  if (!r) return code;
  const label = r.base.label + (r.level ? superscript(r.level) : "");
  return r.prefix ? `${PREFIX_LABELS[r.prefix]} · ${label}` : label;
}

export interface CodeOption {
  code: string;
  label: string;
  categoryId: string;
}

/**
 * The codes a picker offers, in folder order. Prefixes and levels are set on
 * the pill afterwards, so named level rows (HAG²) are left out.
 */
export function codeOptions(vocab: Vocabulary | undefined): CodeOption[] {
  if (!vocab) return [];
  const categoryRank = new Map(vocab.categories.map((c) => [c.id, c.sortOrder]));
  const rank = (c: CodeDef) => categoryRank.get(c.categoryId) ?? Number.MAX_SAFE_INTEGER;
  const isNamedLevel = (c: CodeDef) => {
    const split = splitLevel(c.code);
    return !!split && !!findExact(vocab, split[0]);
  };
  return vocab.codes
    .filter((c) => !isNamedLevel(c))
    .sort((a, b) => rank(a) - rank(b) || a.sortOrder - b.sortOrder)
    .map((c) => ({ code: c.code, label: c.label, categoryId: c.categoryId }));
}

/**
 * True when a search query names one of the item's codes. A family query
 * (HAG, SSG) matches every variant (HAG², T-HAG, SSG³), and a query without
 * a prefix (HAG²) also matches the prefixed forms (T-HAG²).
 */
export function codesMatchQuery(vocab: Vocabulary | undefined, codes: string[], query: string): boolean {
  const key = canonicalForm(query);
  if (!key) return false;
  return codes.some((code) => {
    const r = vocab ? resolveCode(vocab, code) : null;
    if (!r) return code.toUpperCase() === key;
    const stem = r.canonical.slice(r.prefix.length);
    return r.canonical === key || stem === key || r.base.family === key;
  });
}
