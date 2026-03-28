import { invoke } from "@tauri-apps/api/core";
import { getAllAnnotations, getAnnotations, readFileLines } from "$lib/tauri";
import { getDiffState } from "$lib/stores/diff.svelte";
import { getWorkspace } from "$lib/stores/workspace.svelte";
import { getEditor } from "$lib/stores/editor.svelte";
import { getReviewSession } from "$lib/stores/review.svelte";
import type { Annotation, DiffResult, FileAnnotations } from "$lib/types";
import type { FileSnippet } from "$lib/tauri";

type ReviewMode = "changes" | "feedback";

const AGENT_AUTHORS = new Set(["claude", "gpt", "copilot", "gemini", "cursor", "codex", "agent"]);

export interface ReviewFileData {
  filePath: string;
  fileName: string;
  annotations: Annotation[];
  diff: DiffResult | null;
  snippets: Map<string, FileSnippet>; // annotationId -> snippet
}

interface ReviewPageState {
  mode: ReviewMode | null; // null = closed
  activeCardIndex: number;
  files: ReviewFileData[];
  loading: boolean;
  error: string | null;
}

let state = $state<ReviewPageState>({
  mode: null,
  activeCardIndex: 0,
  files: [],
  loading: false,
  error: null,
});

export function getReviewPageState() {
  return state;
}

export function isReviewPageOpen(): boolean {
  return state.mode !== null;
}

export async function openReviewPage(mode: ReviewMode) {
  state.mode = mode;
  state.loading = true;
  state.error = null;
  state.activeCardIndex = 0;
  state.files = [];

  try {
    if (mode === "changes") {
      await loadReviewChanges();
    } else {
      await loadAgentFeedback();
    }
  } catch (e) {
    state.error = e instanceof Error ? e.message : String(e);
  } finally {
    state.loading = false;
  }
}

export function closeReviewPage() {
  state.mode = null;
  state.files = [];
  state.activeCardIndex = 0;
  state.error = null;
}

async function loadReviewChanges() {
  const workspace = getWorkspace();
  const directory = workspace.rootFolders[0];
  if (!directory) {
    state.error = "No workspace open";
    return;
  }

  const session = getReviewSession();
  const editor = getEditor();

  // Determine file list: session files, or current file
  let filePaths: string[] = [];
  if (session.active && session.files.length > 0) {
    filePaths = [...session.files];
  } else if (editor.currentFilePath) {
    filePaths = [editor.currentFilePath];
  } else {
    state.error = "No files to review";
    return;
  }

  const diffState = getDiffState();
  const baseRef = diffState.baseRef || "HEAD";
  const targetRef = diffState.targetRef || "working-tree";

  const results: ReviewFileData[] = [];

  for (const filePath of filePaths) {
    const fileName = filePath.split("/").pop() ?? filePath;

    // Load diff
    let diff: DiffResult | null = null;
    try {
      diff = await invoke<DiffResult>("compute_diff", {
        directory,
        filePath,
        baseRef,
        targetRef,
        algorithm: "patience",
      });
    } catch {
      // File may not have changes — that's ok
    }

    // Load annotations
    let annotations: Annotation[] = [];
    try {
      const sidecar = await getAnnotations(filePath);
      annotations = sidecar.annotations;
    } catch {
      // No annotations for this file
    }

    // Load snippets for annotations (for lines not covered by diff)
    const snippets = new Map<string, FileSnippet>();
    for (const ann of annotations) {
      try {
        const snippet = await readFileLines(filePath, ann.anchor.range.startLine, 3);
        snippets.set(ann.id, snippet);
      } catch {
        // Skip if file not readable
      }
    }

    results.push({ filePath, fileName, annotations, diff, snippets });
  }

  state.files = results;
}

async function loadAgentFeedback() {
  const workspace = getWorkspace();
  const directory = workspace.rootFolders[0];
  if (!directory) {
    state.error = "No workspace open";
    return;
  }

  // Load all annotations across the project
  let allFiles: FileAnnotations[] = [];
  try {
    allFiles = await getAllAnnotations(directory);
  } catch {
    state.error = "Failed to load annotations";
    return;
  }

  // Filter to files with agent annotations
  const results: ReviewFileData[] = [];

  for (const fileGroup of allFiles) {
    const agentAnnotations = fileGroup.annotations.filter((a) =>
      AGENT_AUTHORS.has(a.author.toLowerCase())
    );
    if (agentAnnotations.length === 0) continue;

    const fileName = fileGroup.filePath.split("/").pop() ?? fileGroup.filePath;

    // Load snippets for each annotation
    const snippets = new Map<string, FileSnippet>();
    for (const ann of agentAnnotations) {
      try {
        const snippet = await readFileLines(fileGroup.filePath, ann.anchor.range.startLine, 3);
        snippets.set(ann.id, snippet);
      } catch {
        // Skip
      }
    }

    results.push({
      filePath: fileGroup.filePath,
      fileName,
      annotations: agentAnnotations,
      diff: null,
      snippets,
    });
  }

  state.files = results;
}

// Navigation
export function setActiveCard(index: number) {
  const total = totalCards();
  if (index >= 0 && index < total) {
    state.activeCardIndex = index;
  }
}

export function nextCard() {
  setActiveCard(state.activeCardIndex + 1);
}

export function prevCard() {
  setActiveCard(state.activeCardIndex - 1);
}

function totalCards(): number {
  return state.files.reduce((sum, f) => sum + f.annotations.length, 0);
}

export function getTotalCards(): number {
  return totalCards();
}

export function getResolvedCount(): number {
  return state.files.reduce(
    (sum, f) => sum + f.annotations.filter((a) => a.resolved).length,
    0
  );
}

// Find which file and annotation corresponds to a flat card index
export function getCardAtIndex(index: number): { filePath: string; annotation: Annotation } | null {
  let cursor = 0;
  for (const file of state.files) {
    for (const ann of file.annotations) {
      if (cursor === index) {
        return { filePath: file.filePath, annotation: ann };
      }
      cursor++;
    }
  }
  return null;
}
