import { render, screen } from "@testing-library/svelte";
import { beforeEach, describe, expect, it, vi } from "vitest";
import ReviewWorkspaceHeader from "./ReviewWorkspaceHeader.svelte";
import { activateReviewSession, resetReviewSessionForTests } from "$lib/stores/review.svelte";
import { resetGitHubReviewForTests, setActiveGitHubReviewSessionForTests } from "$lib/stores/githubReview.svelte";

vi.mock("@tauri-apps/plugin-shell", () => ({
  open: vi.fn(async () => {}),
}));

vi.mock("$lib/tauri", () => ({
  listGithubReviewQueue: vi.fn(async () => []),
  openGithubPrReview: vi.fn(),
  resyncGithubPrReview: vi.fn(async () => ({
    id: "gh-1",
    repo: "phin-tech/redpen",
    number: 42,
    title: "Frontend sections",
    body: "",
    url: "https://github.com/phin-tech/redpen/pull/42",
    localRepoPath: "/project",
    worktreePath: "/project",
    baseSha: "base",
    headSha: "head",
    baseRef: "main",
    headRef: "feature",
    changedFiles: [],
    authorLogin: "reviewer",
    viewerLogin: "reviewer-2",
    updatedAt: "2026-03-29T17:00:00Z",
  })),
  discardPendingGithubReviewChanges: vi.fn(async () => ({
    id: "gh-1",
    repo: "phin-tech/redpen",
    number: 42,
    title: "Frontend sections",
    body: "",
    url: "https://github.com/phin-tech/redpen/pull/42",
    localRepoPath: "/project",
    worktreePath: "/project",
    baseSha: "base",
    headSha: "head",
    baseRef: "main",
    headRef: "feature",
    changedFiles: [],
    authorLogin: "reviewer",
    viewerLogin: "reviewer-2",
    updatedAt: "2026-03-29T17:00:00Z",
  })),
  submitGithubPrReview: vi.fn(async () => ({
    session: {
      id: "gh-1",
      repo: "phin-tech/redpen",
      number: 42,
      title: "Frontend sections",
      body: "",
      url: "https://github.com/phin-tech/redpen/pull/42",
      localRepoPath: "/project",
      worktreePath: "/project",
      baseSha: "base",
      headSha: "head",
      baseRef: "main",
      headRef: "feature",
      changedFiles: [],
      authorLogin: "reviewer",
      viewerLogin: "reviewer-2",
      updatedAt: "2026-03-29T17:00:00Z",
    },
    publishedCount: 0,
    replyCount: 0,
  })),
}));

vi.mock("$lib/review", () => ({
  submitReviewVerdict: vi.fn(async () => {}),
}));

describe("ReviewWorkspaceHeader", () => {
  beforeEach(() => {
    resetReviewSessionForTests();
    resetGitHubReviewForTests();
  });

  it("shows local review actions for agent reviews", () => {
    activateReviewSession("local-1", ["/project/file.ts"]);

    render(ReviewWorkspaceHeader, {
      onOpenHelp: vi.fn(),
    });

    expect(screen.getByText("Approve")).toBeTruthy();
    expect(screen.getByText("Request changes")).toBeTruthy();
    expect(screen.queryByText("Submit review")).toBeNull();
  });

  it("shows GitHub review actions for PR reviews", () => {
    setActiveGitHubReviewSessionForTests({
      id: "gh-1",
      repo: "phin-tech/redpen",
      number: 42,
      title: "Frontend sections",
      body: "",
      url: "https://github.com/phin-tech/redpen/pull/42",
      localRepoPath: "/project",
      worktreePath: "/project",
      baseSha: "base",
      headSha: "head",
      baseRef: "main",
      headRef: "feature",
      changedFiles: [],
      authorLogin: "reviewer",
      viewerLogin: "reviewer-2",
      updatedAt: "2026-03-29T17:00:00Z",
    });

    render(ReviewWorkspaceHeader, {
      onOpenHelp: vi.fn(),
    });

    expect(screen.getByText("Resync")).toBeTruthy();
    expect(screen.getByText("Discard pending")).toBeTruthy();
    expect(screen.getByText("Submit review")).toBeTruthy();
  });
});
