import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import CommandPalette from "./CommandPalette.svelte";
import { createCommandRegistry } from "$lib/commands";
import type { AppCommandContext } from "$lib/commands";
import { addRootFolder, resetWorkspaceForTests } from "$lib/stores/workspace.svelte";
import type { WorkspaceFileQueryResponse, WorkspaceIndexStatus } from "$lib/types";

const {
  readDirectoryMock,
  registerWorkspaceRootMock,
  unregisterWorkspaceRootMock,
  getWorkspaceIndexStatusMock,
  queryWorkspaceFilesMock,
  getGitStatusMock,
  openDialogMock,
} = vi.hoisted(() => ({
  readDirectoryMock: vi.fn(),
  registerWorkspaceRootMock: vi.fn(async () => {}),
  unregisterWorkspaceRootMock: vi.fn(async () => {}),
  getWorkspaceIndexStatusMock: vi.fn(async (): Promise<WorkspaceIndexStatus[]> => []),
  queryWorkspaceFilesMock: vi.fn<(query: string, roots?: string[]) => Promise<WorkspaceFileQueryResponse>>(),
  getGitStatusMock: vi.fn(async () => []),
  openDialogMock: vi.fn(),
}));

vi.mock("$lib/tauri", () => ({
  readDirectory: readDirectoryMock,
  registerWorkspaceRoot: registerWorkspaceRootMock,
  unregisterWorkspaceRoot: unregisterWorkspaceRootMock,
  getWorkspaceIndexStatus: getWorkspaceIndexStatusMock,
  queryWorkspaceFiles: queryWorkspaceFilesMock,
  getGitStatus: getGitStatusMock,
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: openDialogMock,
}));

function directoryEntry(path: string) {
  return {
    name: path.split("/").pop() ?? path,
    path,
    isDir: true,
    hasSidecar: false,
  };
}

function readyStatus(root = "/repo", overrides: Partial<WorkspaceIndexStatus> = {}): WorkspaceIndexStatus {
  return {
    root,
    state: "ready",
    indexedCount: 2,
    truncated: false,
    lastUpdated: null,
    error: null,
    ...overrides,
  };
}

function createCommandContext(overrides: Partial<AppCommandContext> = {}): AppCommandContext {
  return {
    openCommandPalette: vi.fn(),
    openFolder: vi.fn(async () => {}),
    openSettings: vi.fn(),
    openAddAnnotation: vi.fn(),
    expandAllFolders: vi.fn(async () => {}),
    collapseAllFolders: vi.fn(),
    toggleShowChangedOnly: vi.fn(),
    hasRoots: () => true,
    canAddAnnotation: () => true,
    hasAnnotations: () => false,
    clearAnnotations: vi.fn(async () => {}),
    reloadAnnotations: vi.fn(async () => {}),
    isMarkdownFile: () => false,
    toggleMarkdownPreview: vi.fn(),
    enterDiffMode: vi.fn(),
    exitDiffMode: vi.fn(),
    hasDiffMode: () => false,
    hasOpenFile: () => true,
    openReviewChanges: vi.fn(),
    openAgentFeedback: vi.fn(),
    isReviewPageOpen: () => false,
    canSubmitReviewVerdict: () => false,
    approveReview: vi.fn(async () => {}),
    requestReviewChanges: vi.fn(async () => {}),
    ...overrides,
  };
}

describe("CommandPalette", () => {
  beforeEach(() => {
    resetWorkspaceForTests();
    readDirectoryMock.mockReset();
    registerWorkspaceRootMock.mockReset();
    unregisterWorkspaceRootMock.mockReset();
    getWorkspaceIndexStatusMock.mockReset();
    queryWorkspaceFilesMock.mockReset();
    getGitStatusMock.mockClear();
    openDialogMock.mockReset();
  });

  it("renders registry commands in default mode", async () => {
    render(CommandPalette, {
      open: true,
      mode: "default",
      onModeChange: vi.fn(),
      onClose: vi.fn(),
      commands: createCommandRegistry(),
      commandContext: createCommandContext(),
      onSelectFile: vi.fn(),
    });

    expect(await screen.findByText("Go to file…")).toBeTruthy();
    expect(screen.getByText("Open settings")).toBeTruthy();
  });

  it("renders backend file-query results in file mode", async () => {
    readDirectoryMock.mockImplementation(async (path: string) =>
      path === "/repo" ? [directoryEntry("/repo/src")] : []
    );
    getWorkspaceIndexStatusMock.mockResolvedValue([readyStatus()]);
    queryWorkspaceFilesMock.mockResolvedValue({
      results: [
        {
          root: "/repo",
          path: "/repo/README.md",
          name: "README.md",
          relativePath: "README.md",
        },
        {
          root: "/repo",
          path: "/repo/src/App.svelte",
          name: "App.svelte",
          relativePath: "src/App.svelte",
        },
      ],
      statuses: [readyStatus()],
    });

    await addRootFolder("/repo");

    render(CommandPalette, {
      open: true,
      mode: "file",
      onModeChange: vi.fn(),
      onClose: vi.fn(),
      commands: createCommandRegistry(),
      commandContext: createCommandContext(),
      onSelectFile: vi.fn(),
    });

    await waitFor(() => {
      expect(screen.getAllByText("README.md").length).toBeGreaterThan(0);
      expect(screen.getByText("App.svelte")).toBeTruthy();
    });
  });

  it("uses the shared file selection callback when a file is chosen", async () => {
    readDirectoryMock.mockResolvedValue([]);
    getWorkspaceIndexStatusMock.mockResolvedValue([readyStatus()]);
    queryWorkspaceFilesMock.mockResolvedValue({
      results: [
        {
          root: "/repo",
          path: "/repo/README.md",
          name: "README.md",
          relativePath: "README.md",
        },
      ],
      statuses: [readyStatus()],
    });

    const onSelectFile = vi.fn(async () => {});
    const onClose = vi.fn();

    await addRootFolder("/repo");

    render(CommandPalette, {
      open: true,
      mode: "file",
      onModeChange: vi.fn(),
      onClose,
      commands: createCommandRegistry(),
      commandContext: createCommandContext(),
      onSelectFile,
    });

    await screen.findAllByText("README.md");
    const [item] = screen.getAllByRole("option");
    item.click();

    await waitFor(() => {
      expect(onSelectFile).toHaveBeenCalledWith("/repo/README.md");
      expect(onClose).toHaveBeenCalled();
    });
  });

  it("closes immediately before async file selection finishes", async () => {
    readDirectoryMock.mockResolvedValue([]);
    getWorkspaceIndexStatusMock.mockResolvedValue([readyStatus()]);
    queryWorkspaceFilesMock.mockResolvedValue({
      results: [
        {
          root: "/repo",
          path: "/repo/README.md",
          name: "README.md",
          relativePath: "README.md",
        },
      ],
      statuses: [readyStatus()],
    });

    let resolveSelection: (() => void) | undefined;
    const onSelectFile = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveSelection = resolve;
        })
    );
    const onClose = vi.fn();

    await addRootFolder("/repo");

    render(CommandPalette, {
      open: true,
      mode: "file",
      onModeChange: vi.fn(),
      onClose,
      commands: createCommandRegistry(),
      commandContext: createCommandContext(),
      onSelectFile,
    });

    await screen.findAllByText("README.md");
    const [item] = screen.getAllByRole("option");
    item.click();

    expect(onClose).toHaveBeenCalledTimes(1);
    expect(onSelectFile).toHaveBeenCalledWith("/repo/README.md");

    resolveSelection?.();
    await waitFor(() => {
      expect(onSelectFile).toHaveBeenCalledTimes(1);
    });
  });

  it("marks file results as selected when navigating with arrow keys", async () => {
    readDirectoryMock.mockResolvedValue([]);
    getWorkspaceIndexStatusMock.mockResolvedValue([readyStatus()]);
    queryWorkspaceFilesMock.mockResolvedValue({
      results: [
        {
          root: "/repo",
          path: "/repo/README.md",
          name: "README.md",
          relativePath: "README.md",
        },
        {
          root: "/repo",
          path: "/repo/src/App.svelte",
          name: "App.svelte",
          relativePath: "src/App.svelte",
        },
      ],
      statuses: [readyStatus()],
    });

    await addRootFolder("/repo");

    render(CommandPalette, {
      open: true,
      mode: "file",
      onModeChange: vi.fn(),
      onClose: vi.fn(),
      commands: createCommandRegistry(),
      commandContext: createCommandContext(),
      onSelectFile: vi.fn(),
    });

    const input = await screen.findByRole("combobox");
    await fireEvent.keyDown(input, { key: "ArrowDown" });

    await waitFor(() => {
      const options = screen.getAllByRole("option");
      expect(options[0]?.getAttribute("aria-selected")).toBe("true");
    });
  });

  it("runs close-on-run registry commands and closes the palette", async () => {
    const onClose = vi.fn();
    const commandContext = createCommandContext();

    render(CommandPalette, {
      open: true,
      mode: "default",
      onModeChange: vi.fn(),
      onClose,
      commands: createCommandRegistry(),
      commandContext,
      onSelectFile: vi.fn(),
    });

    const item = await screen.findByText("Open settings");
    item.click();

    await waitFor(() => {
      expect(commandContext.openSettings).toHaveBeenCalled();
      expect(onClose).toHaveBeenCalled();
    });
  });

  it("keeps the palette open for non-closing commands", async () => {
    const onClose = vi.fn();
    const onModeChange = vi.fn();

    render(CommandPalette, {
      open: true,
      mode: "default",
      onModeChange,
      onClose,
      commands: createCommandRegistry(),
      commandContext: createCommandContext({
        openCommandPalette: onModeChange,
      }),
      onSelectFile: vi.fn(),
    });

    const item = await screen.findByText("Go to file…");
    item.click();

    await waitFor(() => {
      expect(onModeChange).toHaveBeenCalledWith("file");
      expect(onClose).not.toHaveBeenCalled();
    });
  });

  it("shows the truncated index message when file results are partial", async () => {
    readDirectoryMock.mockResolvedValue([]);
    getWorkspaceIndexStatusMock.mockResolvedValue([readyStatus("/repo", { truncated: true })]);
    queryWorkspaceFilesMock.mockResolvedValue({
      results: [
        {
          root: "/repo",
          path: "/repo/src/App.svelte",
          name: "App.svelte",
          relativePath: "src/App.svelte",
        },
      ],
      statuses: [readyStatus("/repo", { truncated: true })],
    });

    await addRootFolder("/repo");

    render(CommandPalette, {
      open: true,
      mode: "file",
      onModeChange: vi.fn(),
      onClose: vi.fn(),
      commands: createCommandRegistry(),
      commandContext: createCommandContext(),
      onSelectFile: vi.fn(),
    });

    expect(await screen.findByText("Some files are omitted due to indexing limits.")).toBeTruthy();
  });
});
