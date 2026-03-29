import {
  discardPendingGithubReviewChanges,
  listGithubReviewQueue,
  openGithubPrReview,
  resyncGithubPrReview,
  submitGithubPrReview,
} from "$lib/tauri";
import type {
  GitHubPrSession,
  GitHubReviewEvent,
  GitHubReviewQueueItem,
  SubmitGitHubReviewResult,
} from "$lib/types";
import { replaceRootFolders } from "./workspace.svelte";
import { clearDiffCache } from "./diff.svelte";

interface GitHubReviewState {
  queue: GitHubReviewQueueItem[];
  loadingQueue: boolean;
  queueError: string | null;
  activeSession: GitHubPrSession | null;
  opening: boolean;
  actionError: string | null;
}

let state = $state<GitHubReviewState>({
  queue: [],
  loadingQueue: false,
  queueError: null,
  activeSession: null,
  opening: false,
  actionError: null,
});

export function getGitHubReviewState() {
  return state;
}

export async function activateGitHubReviewSession(
  session: GitHubPrSession,
  options?: { refreshQueue?: boolean; replaceRoots?: boolean },
) {
  state.activeSession = session;
  state.actionError = null;
  clearDiffCache();

  if (options?.refreshQueue) {
    await loadGitHubReviewQueue();
  }
  if (options?.replaceRoots !== false) {
    await replaceRootFolders([session.worktreePath]);
  }

  return session;
}

export async function loadGitHubReviewQueue() {
  state.loadingQueue = true;
  state.queueError = null;
  try {
    state.queue = await listGithubReviewQueue();
  } catch (error) {
    state.queueError = error instanceof Error ? error.message : String(error);
    state.queue = [];
  } finally {
    state.loadingQueue = false;
  }
}

export async function openGitHubPullRequest(prRef: string, localPathHint?: string) {
  state.opening = true;
  state.actionError = null;
  try {
    const session = await openGithubPrReview(prRef, localPathHint);
    return await activateGitHubReviewSession(session, {
      refreshQueue: true,
      replaceRoots: true,
    });
  } catch (error) {
    state.actionError = error instanceof Error ? error.message : String(error);
    throw error;
  } finally {
    state.opening = false;
  }
}

export async function resyncActiveGitHubReview() {
  if (!state.activeSession) return null;
  state.actionError = null;
  const session = await resyncGithubPrReview(state.activeSession.id);
  return activateGitHubReviewSession(session, { replaceRoots: true });
}

export async function discardActiveGitHubReviewChanges() {
  if (!state.activeSession) return null;
  state.actionError = null;
  const session = await discardPendingGithubReviewChanges(state.activeSession.id);
  return activateGitHubReviewSession(session, { replaceRoots: false });
}

export async function submitActiveGitHubReview(
  event: GitHubReviewEvent,
  summary?: string,
): Promise<SubmitGitHubReviewResult | null> {
  if (!state.activeSession) return null;
  state.actionError = null;
  const result = await submitGithubPrReview(state.activeSession.id, event, summary);
  await activateGitHubReviewSession(result.session, { replaceRoots: false });
  return result;
}

export function clearGitHubReviewSession() {
  state.activeSession = null;
  state.actionError = null;
  clearDiffCache();
}
