import { fireEvent, render, screen, waitFor } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import SettingsDialog from "./SettingsDialog.svelte";

const { getSettingsMock, updateSettingsMock } = vi.hoisted(() => ({
  getSettingsMock: vi.fn(),
  updateSettingsMock: vi.fn(),
}));

vi.mock("$lib/tauri", () => ({
  getSettings: getSettingsMock,
  updateSettings: updateSettingsMock,
}));

describe("SettingsDialog", () => {
  beforeEach(() => {
    getSettingsMock.mockReset();
    updateSettingsMock.mockReset();
  });

  it("loads and saves structured settings including ignored folders", async () => {
    getSettingsMock.mockResolvedValue({
      author: "sam",
      defaultLabels: ["todo", "bug"],
      ignoredFolderNames: ["node_modules", ".venv"],
      defaultCheckoutRoot: "/Users/sam/.config/redpen/checkouts",
      trackedGithubRepos: [],
      notifications: {
        annotationReply: true,
        reviewComplete: true,
        newAnnotation: false,
        deepLink: true,
      },
    });
    updateSettingsMock.mockResolvedValue({
      author: "samwise",
      defaultLabels: ["todo", "question"],
      ignoredFolderNames: ["node_modules", "dist"],
      defaultCheckoutRoot: "/Users/sam/.config/redpen/checkouts",
      trackedGithubRepos: [],
      notifications: {
        annotationReply: true,
        reviewComplete: true,
        newAnnotation: false,
        deepLink: true,
      },
    });

    const onClose = vi.fn();
    render(SettingsDialog, { onClose });

    const authorInput = await screen.findByLabelText("Author name");
    const labelsInput = screen.getByLabelText("Default labels (comma-separated)");
    const ignoredFoldersInput = screen.getByLabelText("Ignored folders (comma-separated)");
    const defaultCheckoutRootInput = screen.getByLabelText("Default checkout location");

    await waitFor(() => {
      expect((authorInput as HTMLInputElement).value).toBe("sam");
      expect((labelsInput as HTMLInputElement).value).toBe("todo, bug");
      expect((ignoredFoldersInput as HTMLInputElement).value).toBe("node_modules, .venv");
      expect((defaultCheckoutRootInput as HTMLInputElement).value).toBe(
        "/Users/sam/.config/redpen/checkouts",
      );
    });

    await fireEvent.input(authorInput, { target: { value: "samwise" } });
    await fireEvent.input(labelsInput, { target: { value: "todo, question" } });
    await fireEvent.input(ignoredFoldersInput, {
      target: { value: " node_modules , dist, " },
    });
    await fireEvent.input(defaultCheckoutRootInput, {
      target: { value: "~/.config/redpen/checkouts" },
    });
    await fireEvent.click(screen.getByRole("button", { name: "Save" }));

    await waitFor(() => {
      expect(updateSettingsMock).toHaveBeenCalledWith({
        author: "samwise",
        defaultLabels: ["todo", "question"],
        ignoredFolderNames: ["node_modules", "dist"],
        defaultCheckoutRoot: "~/.config/redpen/checkouts",
        trackedGithubRepos: [],
        notifications: {
          annotationReply: true,
          reviewComplete: true,
          newAnnotation: false,
          deepLink: true,
        },
      });
      expect(onClose).toHaveBeenCalled();
    });
  });

  it("closes on Escape", async () => {
    getSettingsMock.mockResolvedValue({
      author: "sam",
      defaultLabels: [],
      ignoredFolderNames: [],
      defaultCheckoutRoot: "/Users/sam/.config/redpen/checkouts",
      trackedGithubRepos: [],
      notifications: {
        annotationReply: true,
        reviewComplete: true,
        newAnnotation: false,
        deepLink: true,
      },
    });

    const onClose = vi.fn();
    render(SettingsDialog, { onClose });

    await screen.findByLabelText("Author name");
    await fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalled();
  });
});
