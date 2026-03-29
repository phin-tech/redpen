import { invoke } from "@tauri-apps/api/core";
import { getReviewSession } from "./stores/review.svelte";

export type ReviewVerdict = "approved" | "changes_requested";

export async function submitReviewVerdict(filePath: string, verdict: ReviewVerdict) {
  const reviewSession = getReviewSession();
  await invoke("signal_review_done", {
    filePath,
    verdict,
    sessionId: reviewSession.id ?? undefined,
  });
}
