import { invoke } from "@tauri-apps/api/core";
import type { Annotation, FileEntry, GitFileStatus, SidecarFile } from "./types";

export async function readDirectory(path: string): Promise<FileEntry[]> {
  return invoke("read_directory", { path });
}

export async function readFile(path: string): Promise<string> {
  return invoke("read_file", { path });
}

export async function getAnnotations(filePath: string): Promise<SidecarFile> {
  return invoke("get_annotations", { filePath });
}

export interface CreateAnnotationParams {
  filePath: string;
  kind: string;
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

export async function getGitStatus(directory: string): Promise<GitFileStatus[]> {
  return invoke("get_git_status", { directory });
}

export async function exportAnnotations(filePath: string): Promise<string> {
  return invoke("export_annotations", { filePath });
}

export async function updateSettings(author?: string, defaultLabels?: string[]): Promise<void> {
  return invoke("update_settings", { author, defaultLabels });
}

export async function getSettings(): Promise<[string, string[]]> {
  return invoke("get_settings");
}
