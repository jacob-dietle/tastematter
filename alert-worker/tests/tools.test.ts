import { describe, it, expect } from "vitest";
import { grep } from "../src/tools/grep.js";
import { list } from "../src/tools/list.js";
import { read } from "../src/tools/read.js";
import type { CorpusSnapshot } from "../src/types.js";

function makeCorpus(files?: Record<string, string>): CorpusSnapshot {
  const fileEntries: Record<string, any> = {};
  const allPathsSet = new Set<string>();

  const defaultFiles = files ?? {
    "README.md": "# Hello World\nThis is a test.\nLine 3 with keyword.",
    "docs/guide.md": "---\ntitle: Guide\ntags: [test]\n---\n# Guide\nSome guide content.\nkeyword here too.",
    "docs/api.md": "# API Reference\nNo matches here.",
  };

  for (const [path, content] of Object.entries(defaultFiles)) {
    const size = new TextEncoder().encode(content).length;
    fileEntries[path] = { path, content, size };
    allPathsSet.add(path);

    const parts = path.split("/");
    for (let i = 1; i < parts.length; i++) {
      allPathsSet.add(parts.slice(0, i).join("/") + "/");
    }
  }

  return {
    version: "1.0",
    commit: "abc123",
    fileCount: Object.keys(fileEntries).length,
    totalSize: 0,
    generatedAt: new Date().toISOString(),
    files: fileEntries,
    allPaths: Array.from(allPathsSet).sort(),
  };
}

describe("grep", () => {
  it("finds matches across files", async () => {
    const corpus = makeCorpus();
    const results = await grep(corpus, "keyword");

    expect(results.length).toBe(2);
    expect(results[0].matches.length).toBeGreaterThan(0);
  });

  it("returns empty for no matches", async () => {
    const corpus = makeCorpus();
    const results = await grep(corpus, "nonexistent_pattern_xyz");

    expect(results).toHaveLength(0);
  });

  it("supports case-insensitive search", async () => {
    const corpus = makeCorpus({ "test.md": "Hello WORLD" });
    const results = await grep(corpus, "hello", { caseInsensitive: true });

    expect(results).toHaveLength(1);
  });

  it("throws on invalid regex", async () => {
    const corpus = makeCorpus();
    await expect(grep(corpus, "[invalid")).rejects.toThrow("Invalid regex");
  });

  it("limits results with maxResults", async () => {
    const corpus = makeCorpus();
    const results = await grep(corpus, ".", { maxResults: 1 });

    expect(results.length).toBeLessThanOrEqual(1);
  });

  it("includes context lines", async () => {
    const corpus = makeCorpus();
    const results = await grep(corpus, "keyword", { contextLines: 1 });

    const match = results[0].matches[0];
    expect(match.context).toBeDefined();
  });

  it("sorts by score descending", async () => {
    const corpus = makeCorpus();
    const results = await grep(corpus, "keyword");

    if (results.length >= 2) {
      expect(results[0].score).toBeGreaterThanOrEqual(results[1].score);
    }
  });
});

describe("list", () => {
  it("lists files matching pattern", async () => {
    const corpus = makeCorpus();
    const results = await list(corpus, "**/*.md");

    expect(results.length).toBe(3);
    expect(results.every((r) => r.type === "file")).toBe(true);
  });

  it("lists directories", async () => {
    const corpus = makeCorpus();
    const results = await list(corpus, "docs/", { files: false });

    expect(results.length).toBeGreaterThan(0);
    expect(results[0].type).toBe("directory");
  });

  it("respects maxResults", async () => {
    const corpus = makeCorpus();
    const results = await list(corpus, "**/*", { maxResults: 1 });

    expect(results.length).toBeLessThanOrEqual(1);
  });

  it("throws on missing allPaths", async () => {
    const corpus = makeCorpus();
    (corpus as any).allPaths = undefined;

    await expect(list(corpus, "*")).rejects.toThrow("allPaths");
  });

  it("calculates depth correctly", async () => {
    const corpus = makeCorpus();
    const results = await list(corpus, "**/*.md");

    const readme = results.find((r) => r.path === "README.md");
    const guide = results.find((r) => r.path === "docs/guide.md");

    expect(readme?.depth).toBe(0);
    expect(guide?.depth).toBe(1);
  });
});

describe("read", () => {
  it("reads file content by path", async () => {
    const corpus = makeCorpus();
    const content = await read(corpus, "README.md");

    expect(content).toContain("Hello World");
  });

  it("throws for missing file", async () => {
    const corpus = makeCorpus();
    await expect(read(corpus, "nonexistent.md")).rejects.toThrow("File not found");
  });

  it("normalizes backslashes", async () => {
    const corpus = makeCorpus();
    const content = await read(corpus, "docs\\guide.md");

    expect(content).toContain("Guide");
  });
});
