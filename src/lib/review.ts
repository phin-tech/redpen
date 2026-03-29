import { invoke } from "@tauri-apps/api/core";

export type ReviewVerdict = "approved" | "changes_requested";

export async function submitReviewVerdict(filePath: string, verdict: ReviewVerdict) {
  await invoke("signal_review_done", { filePath, verdict });
}
