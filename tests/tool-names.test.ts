import { describe, it, expect } from "vitest";
import { dotToUnderscore, underscoreToDot, isDotNotation, isUnderscoreNotation, ToolNameRegistry } from "@mcp-rebuild/shared-utils";

describe("dotToUnderscore", () => {
  it("converts single dot", () => {
    expect(dotToUnderscore("workflow.create")).toBe("workflow_create");
  });

  it("converts hyphenated namespace with dot", () => {
    expect(dotToUnderscore("semantic-search.code_search")).toBe("semantic-search_code_search");
  });

  it("preserves camelCase after dot", () => {
    expect(dotToUnderscore("workflow.getActive")).toBe("workflow_getActive");
  });

  it("returns unchanged if no dot", () => {
    expect(dotToUnderscore("ping")).toBe("ping");
  });

  it("converts multiple dots", () => {
    expect(dotToUnderscore("a.b.c")).toBe("a_b_c");
  });

  it("handles empty string", () => {
    expect(dotToUnderscore("")).toBe("");
  });

  it("converts leading dot", () => {
    expect(dotToUnderscore(".name")).toBe("_name");
  });

  it("converts dots in already underscored name", () => {
    expect(dotToUnderscore("a_b.c")).toBe("a_b_c");
  });
});

describe("underscoreToDot", () => {
  it("converts first underscore to dot", () => {
    expect(underscoreToDot("workflow_create")).toBe("workflow.create");
  });

  it("preserves camelCase after underscore", () => {
    expect(underscoreToDot("workflow_getActive")).toBe("workflow.getActive");
  });

  it("handles hyphenated namespace", () => {
    expect(underscoreToDot("semantic-search_code_search")).toBe("semantic-search.code_search");
  });

  it("returns unchanged if no underscore", () => {
    expect(underscoreToDot("ping")).toBe("ping");
  });

  it("converts only first underscore with multiples", () => {
    expect(underscoreToDot("a_b_c")).toBe("a.b_c");
  });

  it("handles empty string", () => {
    expect(underscoreToDot("")).toBe("");
  });

  it("roundtrip dotToUnderscore -> underscoreToDot restores canonical names", () => {
    const roundtrip = (name: string) => underscoreToDot(dotToUnderscore(name));
    expect(roundtrip("workflow.create")).toBe("workflow.create");
    expect(roundtrip("memory.store")).toBe("memory.store");
    expect(roundtrip("semantic-search.code_search")).toBe("semantic-search.code_search");
  });

  it("converts trailing underscore", () => {
    expect(underscoreToDot("name_")).toBe("name.");
  });
});

describe("isDotNotation / isUnderscoreNotation", () => {
  it("detects dot notation", () => {
    expect(isDotNotation("workflow.create")).toBe(true);
    expect(isUnderscoreNotation("workflow.create")).toBe(false);
  });

  it("detects underscore notation", () => {
    expect(isUnderscoreNotation("workflow_create")).toBe(true);
    expect(isDotNotation("workflow_create")).toBe(false);
  });

  it("plain name is neither", () => {
    expect(isDotNotation("ping")).toBe(false);
    expect(isUnderscoreNotation("ping")).toBe(false);
  });

  it("mixed format with both dots and underscores is dot notation", () => {
    expect(isDotNotation("a.b_c")).toBe(true);
    expect(isUnderscoreNotation("a.b_c")).toBe(false);
  });

  it("single character edge cases", () => {
    expect(isDotNotation(".")).toBe(true);
    expect(isUnderscoreNotation(".")).toBe(false);
    expect(isDotNotation("_")).toBe(false);
    expect(isUnderscoreNotation("_")).toBe(true);
    expect(isDotNotation("a")).toBe(false);
    expect(isUnderscoreNotation("a")).toBe(false);
  });
});

describe("ToolNameRegistry", () => {
  it("registers dot name and creates alias", () => {
    const reg = new ToolNameRegistry();
    const result = reg.register("workflow.create");
    expect(result.dot).toBe("workflow.create");
    expect(result.alias).toBe("workflow_create");
  });

  it("resolves dot name to itself", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow.create");
    expect(reg.resolve("workflow.create")).toBe("workflow.create");
  });

  it("resolves underscore alias to dot name", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow.create");
    expect(reg.resolve("workflow_create")).toBe("workflow.create");
  });

  it("getAlias returns underscore version", () => {
    const reg = new ToolNameRegistry();
    reg.register("memory.store");
    expect(reg.getAlias("memory.store")).toBe("memory_store");
  });

  it("getCanonical returns dot version", () => {
    const reg = new ToolNameRegistry();
    reg.register("memory.store");
    expect(reg.getCanonical("memory_store")).toBe("memory.store");
  });

  it("tracks size", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow.create");
    reg.register("memory.store");
    expect(reg.size).toBe(2);
  });

  it("has checks both formats", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow.create");
    expect(reg.has("workflow.create")).toBe(true);
    expect(reg.has("workflow_create")).toBe(true);
    expect(reg.has("nonexistent")).toBe(false);
  });

  it("lists all canonical names and aliases", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow.create");
    reg.register("memory.store");
    expect(reg.canonicalNames).toEqual(["workflow.create", "memory.store"]);
    expect(reg.aliases).toEqual(["workflow_create", "memory_store"]);
  });

  it("resolve with unknown name returns input as-is", () => {
    const reg = new ToolNameRegistry();
    expect(reg.resolve("unknown.tool")).toBe("unknown.tool");
    expect(reg.resolve("unknown_tool")).toBe("unknown_tool");
  });

  it("getAlias with unregistered name returns undefined", () => {
    const reg = new ToolNameRegistry();
    expect(reg.getAlias("nonexistent.create")).toBeUndefined();
  });

  it("getCanonical with unregistered name returns undefined", () => {
    const reg = new ToolNameRegistry();
    expect(reg.getCanonical("nonexistent_create")).toBeUndefined();
  });

  it("duplicate registration overwrites previous", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow.create");
    reg.register("workflow.create");
    expect(reg.size).toBe(1);
  });

  it("registering underscore-format name treats it as canonical", () => {
    const reg = new ToolNameRegistry();
    reg.register("workflow_create");
    expect(reg.resolve("workflow_create")).toBe("workflow_create");
    expect(reg.getCanonical("workflow_create")).toBe("workflow_create");
    expect(reg.getAlias("workflow_create")).toBe("workflow_create");
  });

  it("empty registry canonicalNames and aliases return empty arrays", () => {
    const reg = new ToolNameRegistry();
    expect(reg.canonicalNames).toEqual([]);
    expect(reg.aliases).toEqual([]);
  });

  it("size starts at 0 on empty registry", () => {
    const reg = new ToolNameRegistry();
    expect(reg.size).toBe(0);
  });
});
