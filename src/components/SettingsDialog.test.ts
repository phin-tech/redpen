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

const defaultSettings = {
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
};

describe("SettingsDialog", () => {
  beforeEach(() => {
    getSettingsMock.mockReset();
    updateSettingsMock.mockReset();
  });

  it("loads and saves structured settings including ignored folders", async () => {
    getSettingsMock.mockResolvedValue({ ...defaultSettings });
    updateSettingsMock.mockResolvedValue({ ...defaultSettings, author: "samwise" });

    const onClose = vi.fn();
    render(SettingsDialog, { onClose });

    const authorInput = await screen.findByLabelText("Author name");

    await waitFor(() => {
      expect((authorInput as HTMLInputElement).value).toBe("sam");
    });

    await fireEvent.input(authorInput, { target: { value: "samwise" } });
    await fireEvent.click(screen.getByRole("button", { name: "Save Changes" }));

    await waitFor(() => {
      expect(updateSettingsMock).toHaveBeenCalledWith(
        expect.objectContaining({
          author: "samwise",
          defaultLabels: ["todo", "bug"],
          ignoredFolderNames: ["node_modules", ".venv"],
        }),
      );
      expect(onClose).toHaveBeenCalled();
    });
  });

  it("closes on Escape", async () => {
    getSettingsMock.mockResolvedValue({ ...defaultSettings });

    const onClose = vi.fn();
    render(SettingsDialog, { onClose });

    await screen.findByLabelText("Author name");
    await fireEvent.keyDown(window, { key: "Escape" });

    expect(onClose).toHaveBeenCalled();
  });
});
