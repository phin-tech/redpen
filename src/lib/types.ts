// All types are generated from Rust via ts-rs.
// Regenerate with: task generate:bindings
export {
  type Anchor,
  type AnnotationKind,
  type Choice,
  type SelectionMode,
  type AppSettings,
  type BranchInfo,
  type ChangeKind,
  type CommitInfo,
  type DiffChange,
  type DiffHunk,
  type DiffResult,
  type FileAnnotations,
  type FileEntry,
  type GitFileStatus,
  type NotificationSettings,
  type Range,
  type RefList,
  type SidecarFile,
  type TrackedRepo,
  type UpdateSettingsRequest,
  type WorkspaceFileMatch,
  type WorkspaceFileQueryResponse,
  type WorkspaceIndexState,
  type WorkspaceIndexStatus,
  type GitHubPrSession,
  type GitHubReviewQueueItem,
  type SubmitGitHubReviewResult,
  type GitHubReviewEvent,
} from "./bindings";

// TS-only types (no Rust counterpart)
export type DiffMode = "split" | "unified" | "highlights";

export type GitHubSyncState =
  | "imported"
  | "pendingPublish"
  | "published"
  | "conflict"
  | "localOnly";

export interface GitHubAnnotationMetadata {
  syncState?: GitHubSyncState | null;
  externalCommentId?: string | null;
  externalThreadId?: string | null;
  publishableReason?: string | null;
}

export interface ReviewHistoryItem {
  id: string;
  kind: string;
  status: string;
  title: string;
  subtitle: string;
  updatedAt: string;
  primaryFilePath?: string | null;
  fileCount: number;
  verdict?: string | null;
}

export interface ReviewHistory {
  activeSession?: ReviewHistoryItem | null;
  recentPullRequests: ReviewHistoryItem[];
  recentFiles: ReviewHistoryItem[];
  staleSessions: ReviewHistoryItem[];
}

export interface ResumeReviewSessionResult {
  kind: string;
  sessionId: string;
  projectRoot?: string | null;
  files: string[];
  githubSession?: GitHubPrSession | null;
}

export interface CleanupReviewSessionsResult {
  removedSessions: number;
}

import type { Annotation as GeneratedAnnotation } from "./bindings/Annotation";
import type { GitHubPrSession as GeneratedGitHubPrSession } from "./bindings/GitHubPrSession";
export type Annotation = GeneratedAnnotation & { github?: GitHubAnnotationMetadata | null };
type GitHubPrSession = GeneratedGitHubPrSession;
