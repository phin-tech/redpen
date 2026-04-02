import { beforeEach, describe, expect, it, vi } from "vitest";

const {
  getAllAnnotationsMock,
  getAnnotationsMock,
  getGitStatusMock,
  readFileLinesMock,
  readFileMock,
  cachedInvokeDiffMock,
} = vi.hoisted(() => ({
  getAllAnnotationsMock: vi.fn(),
  getAnnotationsMock: vi.fn(),
  getGitStatusMock: vi.fn(),
  readFileLinesMock: vi.fn(),
  readFileMock: vi.fn(),
  cachedInvokeDiffMock: vi.fn(),
}));

vi.mock("$lib/tauri", () => ({
  getAllAnnotations: getAllAnnotationsMock,
  getAnnotations: getAnnotationsMock,
  getGitStatus: getGitStatusMock,
  readFileLines: readFileLinesMock,
  readFile: readFileMock,
}));

vi.mock("$lib/stores/diff.svelte", () => ({
  getDiffState: () => ({
    baseRef: "HEAD",
    targetRef: "working-tree",
  }),
  cachedInvokeDiff: cachedInvokeDiffMock,
}));

import {
  getReviewPageState,
  openReviewPage,
  resetReviewPageForTests,
} from "./reviewPage.svelte";
import { resetWorkspaceForTests, getWorkspace } from "./workspace.svelte";
import { resetEditorForTests } from "./editor.svelte";
import {
  activateReviewSession,
  resetReviewSessionForTests,
} from "./review.svelte";
import {
  resetGitHubReviewForTests,
} from "./githubReview.svelte";

function emptyDiff() {
  return {
    baseRef: "HEAD",
    targetRef: "working-tree",
    hunks: [],
    insertedLines: [],
    deletedLines: [],
  };
}

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((res) => {
    resolve = res;
  });
  return { promise, resolve };
}

describe("review page store", () => {
  beforeEach(() => {
    resetReviewPageForTests();
    resetWorkspaceForTests();
    resetEditorForTests();
    resetReviewSessionForTests();
    resetGitHubReviewForTests();

    getWorkspace().rootFolders = ["/repo"];

    getAllAnnotationsMock.mockReset();
    getAnnotationsMock.mockReset();
    getGitStatusMock.mockReset();
    readFileLinesMock.mockReset();
    readFileMock.mockReset();
    cachedInvokeDiffMock.mockReset();
  });

  it("starts loading all review files before the first diff resolves", async () => {
    activateReviewSession("session-1", ["/repo/a.ts", "/repo/b.ts"]);

    const diffA = deferred<ReturnType<typeof emptyDiff>>();
    const diffB = deferred<ReturnType<typeof emptyDiff>>();

    cachedInvokeDiffMock.mockImplementation(async (_directory: string, filePath: string) => {
      if (filePath === "/repo/a.ts") return diffA.promise;
      if (filePath === "/repo/b.ts") return diffB.promise;
      throw new Error(`unexpected file ${filePath}`);
    });
    getAnnotationsMock.mockResolvedValue({ annotations: [] });
    readFileMock.mockResolvedValue("");

    const openPromise = openReviewPage("changes");
    await Promise.resolve();

    expect(cachedInvokeDiffMock).toHaveBeenCalledTimes(2);
    expect(cachedInvokeDiffMock.mock.calls.map(([, filePath]) => filePath)).toEqual([
      "/repo/a.ts",
      "/repo/b.ts",
    ]);

    diffA.resolve(emptyDiff());
    diffB.resolve(emptyDiff());

    await openPromise;
  });

  it("reads each file once to build all snippets for that file", async () => {
    activateReviewSession("session-2", ["/repo/file.ts"]);

    cachedInvokeDiffMock.mockResolvedValue(emptyDiff());
    getAnnotationsMock.mockResolvedValue({
      annotations: [
        {
          id: "ann-1",
          body: "first",
          labels: [],
          author: "sam",
          anchor: {
            range: {
              startLine: 2,
              startColumn: 1,
              endLine: 2,
              endColumn: 5,
            },
          },
        },
        {
          id: "ann-2",
          body: "second",
          labels: [],
          author: "sam",
          anchor: {
            range: {
              startLine: 5,
              startColumn: 1,
              endLine: 5,
              endColumn: 4,
            },
          },
        },
      ],
    });
    readFileMock.mockResolvedValue("one\ntwo\nthree\nfour\nfive\nsix");
    readFileLinesMock.mockResolvedValue({
      lines: [],
      startLine: 1,
      totalLines: 0,
    });

    await openReviewPage("changes");

    expect(readFileMock).toHaveBeenCalledTimes(1);
    expect(readFileMock).toHaveBeenCalledWith("/repo/file.ts");
    expect(readFileLinesMock).not.toHaveBeenCalled();

    const file = getReviewPageState().files[0];
    expect(file?.snippets.get("ann-1")).toEqual({
      startLine: 1,
      lines: ["one", "two", "three", "four", "five"],
      totalLines: 6,
    });
    expect(file?.snippets.get("ann-2")).toEqual({
      startLine: 2,
      lines: ["two", "three", "four", "five", "six"],
      totalLines: 6,
    });
  });
});
