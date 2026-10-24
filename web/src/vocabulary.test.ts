import { describe, expect, it } from "vitest";
import type { CodeDef, Vocabulary } from "./api/types";
import { canonicalForm, codeOptions, codesMatchQuery, resolveCode } from "./vocabulary";

function code(code: string, categoryId: string, family: string, acceptsPrefix = false, aliases: string[] = []): CodeDef {
  return { code, label: code, categoryId, family, acceptsPrefix, aliases, sortOrder: 0 };
}

const vocab: Vocabulary = {
  categories: [],
  codes: [
    code("RPG", "RPG", "RPG"),
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

describe("codeOptions", () => {
  it("offers T- and X- forms right after each code that accepts them", () => {
    const codes = codeOptions(vocab).map((o) => o.code);
    expect(codes).toContain("T-HAG");
    expect(codes.slice(codes.indexOf("HAG²"), codes.indexOf("HAG²") + 3)).toEqual([
      "HAG²",
      "T-HAG²",
      "X-HAG²",
    ]);
    expect(codes).not.toContain("T-T-RPG");
    expect(codes).not.toContain("X-RPG");
  });
  it("labels variants and keeps the base category", () => {
    const touch = codeOptions(vocab).find((o) => o.code === "T-HAP");
    expect(touch).toEqual({ code: "T-HAP", label: "觸摸 · HAP", categoryId: "HAG" });
  });
});
