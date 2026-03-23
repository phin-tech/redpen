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

export interface AppSettings {
  author: string;
  defaultLabels: string[];
  ignoredFolderNames: string[];
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
