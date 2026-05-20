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
});
