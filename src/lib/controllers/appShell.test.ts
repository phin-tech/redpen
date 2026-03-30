import { beforeEach, describe, expect, it, vi } from "vitest";
import { createCommandRegistry, findCommand } from "$lib/commands";
import { createAppShellController } from "./appShell.svelte";
import { resetWorkspaceForTests } from "$lib/stores/workspace.svelte";
import { resetEditorForTests, getEditor } from "$lib/stores/editor.svelte";
import { resetAnnotationsForTests } from "$lib/stores/annotations.svelte";
import { resetDiffForTests } from "$lib/stores/diff.svelte";
import { resetReviewSessionForTests } from "$lib/stores/review.svelte";
import { resetReviewPageForTests } from "$lib/stores/reviewPage.svelte";
import { resetGitHubReviewForTests } from "$lib/stores/githubReview.svelte";

const {
  getAnnotationsMock,
  readDirectoryMock,
  readFileMock,
  watchMock,
} = vi.hoisted(() => ({
  getAnnotationsMock: vi.fn(async () => ({ annotations: [] })),
  readDirectoryMock: vi.fn(async () => []),
  readFileMock: vi.fn(async () => "const x = 1;"),
  watchMock: vi.fn(async () => vi.fn()),
}));

vi.mock("$lib/tauri", () => ({
  readDirectory: readDirectoryMock,
  readFile: readFileMock,
  getAnnotations: getAnnotationsMock,
  sendNotification: vi.fn(),
  getSettings: vi.fn(async () => ({ author: "test", defaultLabels: [], ignoredFolderNames: [] })),
  createAnnotation: vi.fn(),
  updateAnnotation: vi.fn(),
  deleteAnnotation: vi.fn(),
  clearAnnotations: vi.fn(),
  getAllAnnotations: vi.fn(async () => []),
  registerWorkspaceRoot: vi.fn(async () => {}),
  unregisterWorkspaceRoot: vi.fn(async () => {}),
  getWorkspaceIndexStatus: vi.fn(async () => []),
  getGitStatus: vi.fn(async () => []),
  listGithubReviewQueue: vi.fn(async () => []),
  openGithubPrReview: vi.fn(),
  resyncGithubPrReview: vi.fn(),
  submitGithubPrReview: vi.fn(),
  discardPendingGithubReviewChanges: vi.fn(),
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
    return null;
  }),
}));

vi.mock("@tauri-apps/plugin-deep-link", () => ({
  onOpenUrl: vi.fn(async () => vi.fn()),
}));

vi.mock("@tauri-apps/plugin-fs", () => ({
  watch: watchMock,
  writeTextFile: vi.fn(),
}));

describe("appShell controller", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    readDirectoryMock.mockClear();
    readFileMock.mockClear();
    getAnnotationsMock.mockClear();
    watchMock.mockClear();
    resetWorkspaceForTests();
    resetEditorForTests();
    resetAnnotationsForTests();
    resetDiffForTests();
    resetReviewSessionForTests();
    resetReviewPageForTests();
    resetGitHubReviewForTests();
  });

  it("opens files through the extracted controller boundary", async () => {
    const controller = createAppShellController();

    await controller.handleFileSelect("/project/file.ts");

    expect(readFileMock).toHaveBeenCalledWith("/project/file.ts");
    expect(getAnnotationsMock).toHaveBeenCalledWith("/project/file.ts");
    expect(getEditor().currentFilePath).toBe("/project/file.ts");
  });

  it("jumps to a file and scrolls after the controller opens it", async () => {
    const scrollToLine = vi.fn();
    const controller = createAppShellController({
      getEditorRef: () => ({
        scrollToLine,
        openSearch: vi.fn(),
        closeSearch: vi.fn(),
        navigateMatch: vi.fn(),
        getView: vi.fn(),
        moveCursorLine: vi.fn(),
        jumpToBoundary: vi.fn(),
        toggleVisualSelection: vi.fn(),
        clearVisualSelection: vi.fn(),
        hasVisualSelection: vi.fn(),
      }),
      jumpToLineDelayMs: 0,
    });

    await controller.handleJumpToFile("/project/file.ts", 12);
    await vi.runAllTimersAsync();

    expect(getEditor().currentFilePath).toBe("/project/file.ts");
    expect(scrollToLine).toHaveBeenCalledWith(12);
  });

  it("keeps command palette commands routed through the same controller behavior", async () => {
    const controller = createAppShellController();
    const commands = createCommandRegistry();

    await findCommand(commands, "navigation.goToFile")?.run(controller.commandContext);
    expect(controller.state.showCommandPalette).toBe(true);
    expect(controller.state.commandPaletteMode).toBe("file");

    controller.openCommandPalette("default");
    await findCommand(commands, "view.openSettings")?.run(controller.commandContext);
    expect(controller.state.showCommandPalette).toBe(false);
    expect(controller.state.showSettings).toBe(true);
  });
});
