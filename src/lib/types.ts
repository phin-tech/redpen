/** Lines are 1-based, columns are 0-based offsets from line start. */
export interface Range {
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export interface TextContextAnchor {
  type: "textContext";
  lineContent: string;
  surroundingLines: string[];
  contentHash: string;
  range: Range;
  lastKnownLine: number;
}

export type Anchor = TextContextAnchor;

export interface Annotation {
  id: string;
  kind: string;
  body: string;
  labels: string[];
  author: string;
  isOrphaned: boolean;
  replyTo: string | null;
  createdAt: string | null;
  updatedAt: string | null;
  anchor: Anchor;
}

export interface SidecarFile {
  version: number;
  sourceFileHash: string;
  annotations: Annotation[];
  metadata: Record<string, unknown>;
}

export interface FileEntry {
  name: string;
  path: string;
  isDir: boolean;
  hasSidecar: boolean;
}

export interface FileAnnotations {
  filePath: string;
  fileName: string;
  annotations: Annotation[];
}

export interface GitFileStatus {
  path: string;
  status: string;
}

export interface NotificationSettings {
  annotationReply: boolean;
  reviewComplete: boolean;
  newAnnotation: boolean;
  deepLink: boolean;
}

export interface AppSettings {
  author: string;
  defaultLabels: string[];
  ignoredFolderNames: string[];
  diffAlgorithm: "patience" | "myers";
  notifications: NotificationSettings;
}

export type WorkspaceIndexState = "indexing" | "ready" | "stale" | "error";

export interface WorkspaceIndexStatus {
  root: string;
  state: WorkspaceIndexState;
  indexedCount: number;
  truncated: boolean;
  lastUpdated: string | null;
  error: string | null;
}

export interface WorkspaceFileMatch {
  root: string;
  path: string;
  name: string;
  relativePath: string;
}

export interface WorkspaceFileQueryResponse {
  results: WorkspaceFileMatch[];
  statuses: WorkspaceIndexStatus[];
}

// Diff types
export type DiffMode = "split" | "unified" | "highlights";
export type ChangeKind = "equal" | "insert" | "delete";

export interface DiffChange {
  kind: ChangeKind;
  oldLine: number | null;
  newLine: number | null;
  content: string;
}

export interface DiffHunk {
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
  changes: DiffChange[];
}

export interface DiffResult {
  baseRef: string;
  targetRef: string;
  hunks: DiffHunk[];
  oldContent: string;
  newContent: string;
}

export interface BranchInfo {
  name: string;
  isCurrent: boolean;
}

export interface CommitInfo {
  sha: string;
  shortMessage: string;
}

export interface RefList {
  branches: BranchInfo[];
  tags: string[];
  recentCommits: CommitInfo[];
}
