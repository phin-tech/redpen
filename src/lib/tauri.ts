import { invoke } from "@tauri-apps/api/core";
import type {
  Annotation,
  AppSettings,
  FileAnnotations,
  FileEntry,
  GitFileStatus,
  SidecarFile,
  WorkspaceFileQueryResponse,
  WorkspaceIndexStatus,
} from "./types";

export async function readDirectory(path: string): Promise<FileEntry[]> {
  return invoke("read_directory", { path });
}

export async function registerWorkspaceRoot(root: string): Promise<void> {
  return invoke("register_workspace_root", { root });
}

export async function unregisterWorkspaceRoot(root: string): Promise<void> {
  return invoke("unregister_workspace_root", { root });
}

export async function getWorkspaceIndexStatus(
  roots?: string[]
): Promise<WorkspaceIndexStatus[]> {
  return invoke("get_workspace_index_status", { request: { roots } });
}

export async function queryWorkspaceFiles(
  query: string,
  roots?: string[],
  limit?: number
): Promise<WorkspaceFileQueryResponse> {
  return invoke("query_workspace_files", { request: { query, roots, limit } });
}

export async function readFile(path: string): Promise<string> {
  return invoke("read_file", { path });
}

export async function getAnnotations(filePath: string): Promise<SidecarFile> {
  return invoke("get_annotations", { filePath });
}

export interface CreateAnnotationParams {
  filePath: string;
  body: string;
  labels: string[];
  startLine: number;
  startColumn: number;
  endLine: number;
  endColumn: number;
}

export async function createAnnotation(request: CreateAnnotationParams): Promise<Annotation> {
  return invoke("create_annotation", { request });
}

export async function updateAnnotation(
  filePath: string, annotationId: string, body?: string, labels?: string[]
): Promise<Annotation> {
  return invoke("update_annotation", { filePath, annotationId, body, labels });
}

export async function deleteAnnotation(filePath: string, annotationId: string): Promise<void> {
  return invoke("delete_annotation", { filePath, annotationId });
}

export async function getAllAnnotations(rootFolder: string): Promise<FileAnnotations[]> {
  return invoke("get_all_annotations", { rootFolder });
}

export async function getGitStatus(directory: string): Promise<GitFileStatus[]> {
  return invoke("get_git_status", { directory });
}

export async function exportAnnotations(filePath: string): Promise<string> {
  return invoke("export_annotations", { filePath });
}

export async function updateSettings(
  patch: Partial<AppSettings>
): Promise<AppSettings> {
  return invoke("update_settings", { request: patch });
}

export async function getSettings(): Promise<AppSettings> {
  return invoke("get_settings");
}
