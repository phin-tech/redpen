import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import EditorPane from "./EditorPane.svelte";

const { readFileMock } = vi.hoisted(() => ({
  readFileMock: vi.fn(),
}));

vi.mock("$lib/tauri", () => ({
  readFile: readFileMock,
  readDirectory: vi.fn(),
  registerWorkspaceRoot: vi.fn(async () => {}),
  unregisterWorkspaceRoot: vi.fn(async () => {}),
  getWorkspaceIndexStatus: vi.fn(async () => []),
  queryWorkspaceFiles: vi.fn(),
  getAnnotations: vi.fn(async () => null),
  createAnnotation: vi.fn(),
  updateAnnotation: vi.fn(),
  deleteAnnotation: vi.fn(),
  getAllAnnotations: vi.fn(async () => []),
  getGitStatus: vi.fn(async () => []),
  getGitRoot: vi.fn(async () => null),
  getSettings: vi.fn(async () => ({ author: "", defaultLabels: [], ignoredFolderNames: [] })),
  updateSettings: vi.fn(),
  exportAnnotations: vi.fn(),
  listGithubReviewQueue: vi.fn(async () => []),
  openGithubPrReview: vi.fn(),
  resyncGithubPrReview: vi.fn(),
  submitGithubPrReview: vi.fn(),
  discardPendingGithubReviewChanges: vi.fn(),
  getReviewHistory: vi.fn(async () => ({
    activeSession: null,
    recentPullRequests: [],
    recentFiles: [],
    staleSessions: [],
  })),
  resumeReviewSession: vi.fn(),
  cleanupStaleReviewSessions: vi.fn(async () => ({ removedSessions: 0 })),
}));

vi.mock("mermaid", () => ({
  default: {
    initialize: vi.fn(),
    run: vi.fn(),
  },
}));

import { openFile } from "$lib/stores/editor.svelte";
import { addRootFolder, resetWorkspaceForTests } from "$lib/stores/workspace.svelte";
import { resetGitHubReviewForTests } from "$lib/stores/githubReview.svelte";
import { resetReviewPageForTests } from "$lib/stores/reviewPage.svelte";
import { resetReviewSessionForTests } from "$lib/stores/review.svelte";
import { resetDiffForTests } from "$lib/stores/diff.svelte";
import { resetEditorForTests } from "$lib/stores/editor.svelte";
import { resetAnnotationsForTests } from "$lib/stores/annotations.svelte";

describe("EditorPane", () => {
  beforeEach(() => {
    readFileMock.mockReset();
    resetWorkspaceForTests();
    resetEditorForTests();
    resetAnnotationsForTests();
    resetDiffForTests();
    resetReviewSessionForTests();
    resetReviewPageForTests();
    resetGitHubReviewForTests();
  });

  it("renders the editor pane container", async () => {
    readFileMock.mockResolvedValue("const x = 1;");
    await openFile("/project/file.ts");
    await addRootFolder("/project");

    const { container } = render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    expect(container.querySelector(".editor-pane")).toBeTruthy();
  });

  // Toolbar tests (Source/Preview, Code/Review/PR tabs) removed —
  // WorkspaceToolbar moved to App.svelte global header
});
