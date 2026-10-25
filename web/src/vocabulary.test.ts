import { describe, expect, it } from "vitest";
import type { CodeDef, Vocabulary } from "./api/types";
import {
  canonicalForm,
  codeOptions,
  codeParts,
  codesMatchQuery,
  codeTitle,
  composeCode,
  resolveCode,
} from "./vocabulary";

function code(code: string, categoryId: string, family: string, acceptsPrefix = false, aliases: string[] = []): CodeDef {
  return { code, label: code, categoryId, family, acceptsPrefix, aliases, sortOrder: 0 };
}

const vocab: Vocabulary = {
  categories: [],
  codes: [
    code("RPG", "RPG", "RPG"),
    code("EDU", "SIM", "EDU"),
    code("T-RPG", "SLG", "T-RPG"),
    code("ERG", "PZL", "ERG", false, ["H-ERG"]),
    code("HAG", "HAG", "HAG", true),
    code("HAG²", "HAG", "HAG", true),
    code("HAP", "HAG", "HAP", true),
  ],
};

describe("canonicalForm", () => {
  it("uppercases and turns trailing digits into superscripts", () => {
    expect(canonicalForm(" hag2 ")).toBe("HAG²");
    expect(canonicalForm("T-hag4")).toBe("T-HAG⁴");
    expect(canonicalForm("42")).toBe("42");
  });
});

describe("resolveCode", () => {
  it("resolves aliases, prefixes and exact codes", () => {
    expect(resolveCode(vocab, "h-erg")?.canonical).toBe("ERG");
    expect(resolveCode(vocab, "t-hag2")?.canonical).toBe("T-HAG²");
    expect(resolveCode(vocab, "t-rpg")?.base.categoryId).toBe("SLG");
    expect(resolveCode(vocab, "X-RPG")).toBeNull();
  });
});

describe("codesMatchQuery", () => {
  it("matches a family against all of its variants", () => {
    expect(codesMatchQuery(vocab, ["T-HAG²"], "hag")).toBe(true);
    expect(codesMatchQuery(vocab, ["HAP"], "hag")).toBe(false);
  });
  it("matches a base code against its prefixed forms only", () => {
    expect(codesMatchQuery(vocab, ["T-HAG²"], "HAG2")).toBe(true);
    expect(codesMatchQuery(vocab, ["HAG"], "HAG2")).toBe(false);
  });
  it("does not confuse T-RPG with RPG", () => {
    expect(codesMatchQuery(vocab, ["T-RPG"], "rpg")).toBe(false);
  });
});

describe("levels", () => {
  it("lets any code take a ² to ⁹ level", () => {
    const r = resolveCode(vocab, "rpg3");
    expect(r?.canonical).toBe("RPG³");
    expect(r?.base.categoryId).toBe("RPG");
    expect(resolveCode(vocab, "t-hap4")?.canonical).toBe("T-HAP⁴");
    expect(resolveCode(vocab, "RPG1")).toBeNull();
    expect(resolveCode(vocab, "RPG23")).toBeNull();
  });
  it("prefers a named level row over the generic reading", () => {
    expect(resolveCode(vocab, "HAG²")).toMatchObject({ level: null, canonical: "HAG²" });
    expect(resolveCode(vocab, "HAG³")).toMatchObject({ level: 3, canonical: "HAG³" });
  });
  it("labels a level after the base code", () => {
    expect(codeTitle(vocab, "T-RPG³")).toBe("T-RPG³");
    expect(codeTitle(vocab, "X-HAG⁴")).toBe("局部/斷面 · HAG⁴");
  });
  it("matches a family query against levelled codes", () => {
    expect(codesMatchQuery(vocab, ["RPG³"], "rpg")).toBe(true);
    expect(codesMatchQuery(vocab, ["T-HAP²"], "hap2")).toBe(true);
  });
});

describe("codeParts and composeCode", () => {
  it("splits named level rows like any other level", () => {
    expect(codeParts(vocab, "T-HAG²")).toEqual({ prefix: "T-", root: "HAG", level: 2 });
    expect(codeParts(vocab, "T-RPG")).toEqual({ prefix: "", root: "T-RPG", level: null });
  });
  it("only composes combinations the vocabulary accepts", () => {
    expect(composeCode(vocab, { prefix: "X-", root: "HAG", level: 3 })).toBe("X-HAG³");
    expect(composeCode(vocab, { prefix: "T-", root: "RPG", level: null })).toBe("T-RPG");
    expect(composeCode(vocab, { prefix: "T-", root: "EDU", level: 2 })).toBeNull();
  });
});

describe("codeOptions", () => {
  it("offers each code once and leaves named levels to the pill", () => {
    const codes = codeOptions(vocab).map((o) => o.code);
    expect(codes).toContain("HAG");
    expect(codes).not.toContain("HAG²");
    expect(codes).not.toContain("T-HAG");
  });
});
