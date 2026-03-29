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

describe("EditorPane", () => {
  beforeEach(() => {
    readFileMock.mockReset();
    resetWorkspaceForTests();
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
});
