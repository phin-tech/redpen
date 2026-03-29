import { render, screen, fireEvent } from "@testing-library/svelte";
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

// Mock mermaid to avoid DOM rendering issues in jsdom
vi.mock("mermaid", () => ({
  default: {
    initialize: vi.fn(),
    run: vi.fn(),
  },
}));

import { openFile } from "$lib/stores/editor.svelte";
import { addRootFolder, resetWorkspaceForTests } from "$lib/stores/workspace.svelte";
import { setActiveGitHubReviewSessionForTests, resetGitHubReviewForTests } from "$lib/stores/githubReview.svelte";
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

  it("does not show toggle bar for non-markdown files", async () => {
    readFileMock.mockResolvedValue("const x = 1;");
    await openFile("/project/file.ts");
    await addRootFolder("/project");

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    expect(screen.queryByText("Source")).toBeNull();
    expect(screen.queryByText("Preview")).toBeNull();
  });

  it("shows toggle bar for markdown files", async () => {
    readFileMock.mockResolvedValue("# Hello");
    await openFile("/project/readme.md");
    await addRootFolder("/project");

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    expect(screen.getByText("Source")).toBeTruthy();
    expect(screen.getByText("Preview")).toBeTruthy();
  });

  it("defaults to source view for markdown files", async () => {
    readFileMock.mockResolvedValue("# Hello");
    await openFile("/project/readme.md");
    await addRootFolder("/project");

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    const sourceBtn = screen.getByText("Source");
    expect(sourceBtn.closest("button")?.classList.contains("active")).toBe(true);
  });

  it("switches to preview when Preview button is clicked", async () => {
    readFileMock.mockResolvedValue("# Hello World\n\nSome text.");
    await openFile("/project/readme.md");
    await addRootFolder("/project");

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    const previewBtn = screen.getByText("Preview");
    await fireEvent.click(previewBtn);

    // Preview should now be active
    expect(previewBtn.closest("button")?.classList.contains("active")).toBe(true);

    // Rendered markdown should be visible
    expect(screen.getByText("Hello World")).toBeTruthy();
    expect(screen.getByText("Some text.")).toBeTruthy();
  });

  it("renders mermaid code blocks as pre.mermaid in preview", async () => {
    readFileMock.mockResolvedValue("```mermaid\ngraph TD;\n  A-->B;\n```");
    await openFile("/project/diagram.md");
    await addRootFolder("/project");

    const { container } = render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    await fireEvent.click(screen.getByText("Preview"));

    const mermaidPre = container.querySelector("pre.mermaid");
    expect(mermaidPre).toBeTruthy();
    expect(mermaidPre?.textContent).toContain("graph TD;");
  });

  it("switches back to source view when Source button is clicked", async () => {
    readFileMock.mockResolvedValue("# Hello");
    await openFile("/project/readme.md");
    await addRootFolder("/project");

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    // Go to preview
    await fireEvent.click(screen.getByText("Preview"));
    expect(screen.getByText("Preview").closest("button")?.classList.contains("active")).toBe(true);

    // Go back to source
    await fireEvent.click(screen.getByText("Source"));
    expect(screen.getByText("Source").closest("button")?.classList.contains("active")).toBe(true);
  });

  it("switches between code and review toolbar tabs", async () => {
    readFileMock.mockResolvedValue("const x = 1;");
    await openFile("/project/file.ts");
    await addRootFolder("/project");

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    const reviewTab = screen.getByText("Review");
    await fireEvent.click(reviewTab);
    expect(reviewTab.closest("button")?.classList.contains("active")).toBe(true);

    const codeTab = screen.getByText("Code");
    await fireEvent.click(codeTab);
    expect(codeTab.closest("button")?.classList.contains("active")).toBe(true);
  });

  it("shows and activates the PR tab for GitHub review sessions", async () => {
    readFileMock.mockResolvedValue("const x = 1;");
    await openFile("/project/file.ts");
    await addRootFolder("/project");
    setActiveGitHubReviewSessionForTests({
      id: "gh-1",
      repo: "phin-tech/redpen",
      number: 42,
      title: "Frontend sections",
      body: "",
      url: "https://github.com/phin-tech/redpen/pull/42",
      localRepoPath: "/project",
      worktreePath: "/project",
      baseSha: "base",
      headSha: "head",
      baseRef: "main",
      headRef: "feature",
      changedFiles: [],
      authorLogin: "reviewer",
      viewerLogin: "reviewer-2",
      updatedAt: "2026-03-29T17:00:00Z",
    });

    render(EditorPane, {
      onSelectionChange: vi.fn(),
    });

    const prTab = screen.getByText("PR");
    await fireEvent.click(prTab);
    expect(prTab.closest("button")?.classList.contains("active")).toBe(true);
  });
});
