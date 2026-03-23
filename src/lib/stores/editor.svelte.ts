import { readFile } from "$lib/tauri";

interface EditorState {
  currentFilePath: string | null;
  content: string;
  loading: boolean;
  showPreview: boolean;
}

let state = $state<EditorState>({
  currentFilePath: null,
  content: "",
  loading: false,
  showPreview: false,
});

export function getEditor() {
  return state;
}

export async function openFile(path: string) {
  state.loading = true;
  state.currentFilePath = path;
  state.showPreview = false;
  try {
    state.content = await readFile(path);
  } catch (e) {
    console.error("Failed to read file:", e);
    state.content = `Error reading file: ${e}`;
  } finally {
    state.loading = false;
  }
}

export function getFileExtension(): string {
  if (!state.currentFilePath) return "";
  const parts = state.currentFilePath.split(".");
  return parts.length > 1 ? parts[parts.length - 1] : "";
}

export function isMarkdownFile(): boolean {
  return getFileExtension() === "md";
}

export function getShowPreview(): boolean {
  return state.showPreview;
}

export function togglePreview() {
  state.showPreview = !state.showPreview;
}
