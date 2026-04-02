import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  cachedInvokeDiff,
  clearDiffCache,
  resetDiffForTests,
} from "./diff.svelte";

function diffFor(filePath: string) {
  return {
    baseRef: "HEAD",
    targetRef: "working-tree",
    hunks: [
      {
        oldStart: 1,
        oldCount: 0,
        newStart: 1,
        newCount: 1,
        changes: [
          {
            kind: "insert",
            oldLine: null,
            newLine: 1,
            content: filePath,
          },
        ],
      },
    ],
    insertedLines: [1],
    deletedLines: [],
  };
}

describe("diff store cache", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockImplementation(async (_command: string, args: { filePath: string }) => {
      return diffFor(args.filePath);
    });
    resetDiffForTests();
    clearDiffCache();
  });

  it("reuses cached diff results for the same request", async () => {
    const first = await cachedInvokeDiff("/repo", "/repo/a.ts", "HEAD", "working-tree", "patience");
    const second = await cachedInvokeDiff("/repo", "/repo/a.ts", "HEAD", "working-tree", "patience");

    expect(first).toEqual(second);
    expect(invokeMock).toHaveBeenCalledTimes(1);
  });

  it("evicts older entries once the cache reaches its max size", async () => {
    const cacheSize = 128;

    for (let index = 0; index <= cacheSize; index += 1) {
      await cachedInvokeDiff(
        "/repo",
        `/repo/file-${index}.ts`,
        "HEAD",
        "working-tree",
        "patience",
      );
    }

    await cachedInvokeDiff("/repo", "/repo/file-0.ts", "HEAD", "working-tree", "patience");

    expect(invokeMock).toHaveBeenCalledTimes(cacheSize + 2);
  });
});
