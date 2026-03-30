import { render } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App.svelte";
import { resetWorkspaceForTests } from "$lib/stores/workspace.svelte";
import { resetEditorForTests } from "$lib/stores/editor.svelte";
import { resetAnnotationsForTests } from "$lib/stores/annotations.svelte";
import { resetDiffForTests } from "$lib/stores/diff.svelte";
import { resetReviewSessionForTests } from "$lib/stores/review.svelte";
import { resetReviewPageForTests } from "$lib/stores/reviewPage.svelte";
import { resetGitHubReviewForTests } from "$lib/stores/githubReview.svelte";

vi.mock("$lib/tauri", () => ({
  readDirectory: vi.fn(async () => []),
  readFile: vi.fn(async () => ""),
  registerWorkspaceRoot: vi.fn(async () => {}),
  unregisterWorkspaceRoot: vi.fn(async () => {}),
  getWorkspaceIndexStatus: vi.fn(async () => []),
  queryWorkspaceFiles: vi.fn(async () => ({ results: [], statuses: [] })),
  getAnnotations: vi.fn(async () => ({ annotations: [] })),
  createAnnotation: vi.fn(),
  updateAnnotation: vi.fn(),
  deleteAnnotation: vi.fn(),
  clearAnnotations: vi.fn(),
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
  sendNotification: vi.fn(),
  readFileLines: vi.fn(async () => ({ startLine: 1, lines: [] })),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(),
  save: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: vi.fn(() => ({
    onDragDropEvent: vi.fn(async () => vi.fn()),
  })),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === "get_pending_deep_links") return [];
    if (command === "get_git_root") return null;
    if (command === "export_annotations") return "";
    if (command === "list_refs") return { branches: [], tags: [], commits: [] };
    return null;
  }),
}));

vi.mock("@tauri-apps/plugin-deep-link", () => ({
  onOpenUrl: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  watch: vi.fn(async () => vi.fn()),
  writeTextFile: vi.fn(),
}));

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn(async () => {}),
}));

vi.mock("@tauri-apps/plugin-log", () => ({
  info: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
}));

vi.mock("mermaid", () => ({
  default: {
    initialize: vi.fn(),
    run: vi.fn(),
  },
}));

describe("App", () => {
  beforeEach(() => {
    resetWorkspaceForTests();
    resetEditorForTests();
    resetAnnotationsForTests();
    resetDiffForTests();
    resetReviewSessionForTests();
    resetReviewPageForTests();
    resetGitHubReviewForTests();
  });

  it("renders the workspace shell with center panel (sidebars hidden when no workspace open)", () => {
    const { container } = render(App);

    expect(container.querySelector(".workspace-shell")).toBeTruthy();
    // With no workspace roots, only the center panel renders (sidebars hidden)
    expect(container.querySelectorAll(".app-panel")).toHaveLength(1);
    expect(container.querySelector(".app-panel-workspace")).toBeTruthy();
  });
});
