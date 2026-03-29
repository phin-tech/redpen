import { getAllAnnotations, getAnnotations, getGitStatus, readFileLines } from "$lib/tauri";

import { getDiffState, cachedInvokeDiff } from "$lib/stores/diff.svelte";
import { getWorkspace } from "$lib/stores/workspace.svelte";
import { getEditor } from "$lib/stores/editor.svelte";
import { getReviewSession } from "$lib/stores/review.svelte";
import { getGitHubReviewState } from "$lib/stores/githubReview.svelte";
import type { Annotation, DiffResult, FileAnnotations } from "$lib/types";
import type { FileSnippet } from "$lib/tauri";

type ReviewMode = "changes" | "feedback";
type ReviewScope = "session" | "all-changes";

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
  scope: ReviewScope;
  activeCardIndex: number;
  files: ReviewFileData[];
  loading: boolean;
  error: string | null;
}

let state = $state<ReviewPageState>({
  mode: null,
  scope: "session",
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

export async function toggleScope() {
  state.scope = state.scope === "session" ? "all-changes" : "session";
  if (state.mode === "changes") {
    state.loading = true;
    state.error = null;
    state.activeCardIndex = 0;
    state.files = [];
    try {
      await loadReviewChanges();
    } catch (e) {
      state.error = e instanceof Error ? e.message : String(e);
    } finally {
      state.loading = false;
    }
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
  console.log("[ReviewPage] directory:", directory);
  if (!directory) {
    state.error = "No workspace open";
    return;
  }

  const session = getReviewSession();
  const githubReview = getGitHubReviewState();
  const editor = getEditor();
  console.log("[ReviewPage] session.active:", session.active, "session.files:", session.files);
  console.log("[ReviewPage] editor.currentFilePath:", editor.currentFilePath);

  // Determine file list based on scope
  let filePaths: string[] = [];
  if (
    githubReview.activeSession &&
    githubReview.activeSession.worktreePath === directory
  ) {
    filePaths = githubReview.activeSession.changedFiles.map(
      (relativePath) => `${directory}/${relativePath}`
    );
  } else if (state.scope === "all-changes") {
    try {
      const statuses = await getGitStatus(directory);
      filePaths = statuses.map((s) => s.path.startsWith("/") ? s.path : `${directory}/${s.path}`);
    } catch (e) {
      console.warn("[ReviewPage] getGitStatus failed", e);
      state.error = "Failed to get git status";
      return;
    }
  } else if (session.active && session.files.length > 0) {
    filePaths = [...session.files];
  } else if (editor.currentFilePath) {
    filePaths = [editor.currentFilePath];
  }

  if (filePaths.length === 0) {
    state.error = state.scope === "all-changes" ? "No changed files" : "No files to review";
    return;
  }

  const diffState = getDiffState();
  const baseRef =
    githubReview.activeSession && githubReview.activeSession.worktreePath === directory
      ? githubReview.activeSession.baseSha
      : diffState.baseRef || "HEAD";
  const targetRef =
    githubReview.activeSession && githubReview.activeSession.worktreePath === directory
      ? githubReview.activeSession.headSha
      : diffState.targetRef || "working-tree";
  console.log("[ReviewPage] filePaths:", filePaths, "baseRef:", baseRef, "targetRef:", targetRef);

  const results: ReviewFileData[] = [];

  for (const filePath of filePaths) {
    const fileName = filePath.split("/").pop() ?? filePath;
    console.log("[ReviewPage] processing file:", filePath);

    // Load diff
    let diff: DiffResult | null = null;
    try {
      console.log("[ReviewPage] calling compute_diff...");
      diff = await cachedInvokeDiff(directory, filePath, baseRef, targetRef, "patience");
      console.log("[ReviewPage] compute_diff done, hunks:", diff?.hunks?.length ?? 0);
    } catch (e) {
      console.warn("[ReviewPage] compute_diff failed for", filePath, e);
    }

    // Load annotations
    let annotations: Annotation[] = [];
    try {
      console.log("[ReviewPage] calling getAnnotations...");
      const sidecar = await getAnnotations(filePath);
      annotations = sidecar.annotations;
      console.log("[ReviewPage] getAnnotations done, count:", annotations.length);
    } catch (e) {
      console.warn("[ReviewPage] getAnnotations failed for", filePath, e);
    }

    // Load snippets for annotations
    const snippets = new Map<string, FileSnippet>();
    for (const ann of annotations) {
      try {
        const snippet = await readFileLines(filePath, ann.anchor.range.startLine, 3);
        snippets.set(ann.id, snippet);
      } catch (e) {
        console.warn("[ReviewPage] readFileLines failed for", ann.id, e);
      }
    }
    console.log("[ReviewPage] snippets loaded for", filePath);

    results.push({ filePath, fileName, annotations, diff, snippets });
  }

  console.log("[ReviewPage] all files processed, count:", results.length);
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

// Navigation — flat list includes annotation cards + diff-only file entries
function totalItems(): number {
  let count = 0;
  for (const file of state.files) {
    const roots = file.annotations.filter(a => !a.replyTo).length;
    count += roots > 0 ? roots : (file.diff ? 1 : 0);
  }
  return count;
}

export function getTotalItems(): number {
  return totalItems();
}

export function setActiveCard(index: number) {
  const total = totalItems();
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

export function getAnnotationCount(): number {
  return state.files.reduce((sum, f) => sum + f.annotations.filter(a => !a.replyTo).length, 0);
}

export function getResolvedCount(): number {
  return state.files.reduce(
    (sum, f) => sum + f.annotations.filter((a) => !a.replyTo && a.resolved).length,
    0
  );
}

// Find which file and annotation corresponds to a flat index
export function getCardAtIndex(index: number): { filePath: string; annotation: Annotation | null } | null {
  let cursor = 0;
  for (const file of state.files) {
    const roots = file.annotations.filter(a => !a.replyTo);
    if (roots.length > 0) {
      for (const ann of roots) {
        if (cursor === index) return { filePath: file.filePath, annotation: ann };
        cursor++;
      }
    } else if (file.diff) {
      if (cursor === index) return { filePath: file.filePath, annotation: null };
      cursor++;
    }
  }
  return null;
}
