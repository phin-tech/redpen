import { getAllAnnotations, getAnnotations, getGitStatus, readFile } from "$lib/tauri";

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
  if (!directory) {
    state.error = "No workspace open";
    return;
  }

  const session = getReviewSession();
  const githubReview = getGitHubReviewState();
  const editor = getEditor();

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
  state.files = await Promise.all(
    filePaths.map((filePath) =>
      loadReviewFileData(directory, filePath, baseRef, targetRef)
    )
  );
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
  const results: Array<ReviewFileData | null> = await Promise.all(
    allFiles.map(async (fileGroup) => {
      const agentAnnotations = fileGroup.annotations.filter((a) =>
        AGENT_AUTHORS.has(a.author.toLowerCase())
      );
      if (agentAnnotations.length === 0) return null;

      return {
        filePath: fileGroup.filePath,
        fileName: fileGroup.filePath.split("/").pop() ?? fileGroup.filePath,
        annotations: agentAnnotations,
        diff: null,
        snippets: await buildSnippetsForAnnotations(fileGroup.filePath, agentAnnotations),
      } satisfies ReviewFileData;
    })
  );

  state.files = results.filter((result): result is ReviewFileData => result !== null);
}

async function loadReviewFileData(
  directory: string,
  filePath: string,
  baseRef: string,
  targetRef: string,
): Promise<ReviewFileData> {
  const fileName = filePath.split("/").pop() ?? filePath;

  const [diff, annotations] = await Promise.all([
    loadDiff(directory, filePath, baseRef, targetRef),
    loadAnnotationsForFile(filePath),
  ]);

  return {
    filePath,
    fileName,
    annotations,
    diff,
    snippets: await buildSnippetsForAnnotations(filePath, annotations),
  };
}

async function loadDiff(
  directory: string,
  filePath: string,
  baseRef: string,
  targetRef: string,
): Promise<DiffResult | null> {
  try {
    return await cachedInvokeDiff(directory, filePath, baseRef, targetRef, "patience");
  } catch {
    return null;
  }
}

async function loadAnnotationsForFile(filePath: string): Promise<Annotation[]> {
  try {
    const sidecar = await getAnnotations(filePath);
    return sidecar.annotations;
  } catch {
    return [];
  }
}

async function buildSnippetsForAnnotations(
  filePath: string,
  annotations: Annotation[],
): Promise<Map<string, FileSnippet>> {
  const snippets = new Map<string, FileSnippet>();
  if (annotations.length === 0) return snippets;

  let content: string;
  try {
    content = await readFile(filePath);
  } catch {
    return snippets;
  }

  const allLines = content.length === 0
    ? []
    : content.replace(/\r?\n$/, "").split(/\r?\n/);
  const totalLines = allLines.length;

  for (const annotation of annotations) {
    const centerLine = annotation.anchor.range.startLine;
    const startIndex = centerLine <= 4 ? 0 : centerLine - 4;
    const endIndex = Math.min(centerLine + 3, totalLines);

    snippets.set(annotation.id, {
      startLine: startIndex + 1,
      lines: allLines.slice(startIndex, endIndex),
      totalLines,
    });
  }

  return snippets;
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

export function setReviewPageStateForTests(
  mode: ReviewMode | null,
  options?: {
    scope?: ReviewScope;
    files?: ReviewFileData[];
    loading?: boolean;
    error?: string | null;
  },
) {
  state.mode = mode;
  state.scope = options?.scope ?? "session";
  state.activeCardIndex = 0;
  state.files = options?.files ?? [];
  state.loading = options?.loading ?? false;
  state.error = options?.error ?? null;
}

export function resetReviewPageForTests() {
  state.mode = null;
  state.scope = "session";
  state.activeCardIndex = 0;
  state.files = [];
  state.loading = false;
  state.error = null;
}
