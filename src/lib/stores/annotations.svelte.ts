import { getAnnotations, createAnnotation, updateAnnotation, deleteAnnotation, getAllAnnotations } from "$lib/tauri";
import type { Annotation, FileAnnotations, SidecarFile } from "$lib/types";

type AnnotationFilter = "all" | "comment" | "lineNote" | "label";
type SidebarView = "file" | "project";

interface AnnotationsState {
  sidecar: SidecarFile | null;
  selectedAnnotationId: string | null;
  filter: AnnotationFilter;
  sidebarView: SidebarView;
  projectAnnotations: FileAnnotations[];
  projectAnnotationsLoading: boolean;
}

let state = $state<AnnotationsState>({
  sidecar: null,
  selectedAnnotationId: null,
  filter: "all",
  sidebarView: "file",
  projectAnnotations: [],
  projectAnnotationsLoading: false,
});

export function getAnnotationsState() {
  return state;
}

export async function loadAnnotations(filePath: string) {
  try {
    state.sidecar = await getAnnotations(filePath);
  } catch {
    state.sidecar = null;
  }
}

export async function loadProjectAnnotations(rootFolder: string) {
  state.projectAnnotationsLoading = true;
  try {
    state.projectAnnotations = await getAllAnnotations(rootFolder);
  } catch {
    state.projectAnnotations = [];
  }
  state.projectAnnotationsLoading = false;
}

export function setSidebarView(view: SidebarView) {
  state.sidebarView = view;
}

export function setFilter(filter: AnnotationFilter) {
  state.filter = filter;
}

export function sortedAnnotations(): Annotation[] {
  if (!state.sidecar) return [];
  let annotations = [...state.sidecar.annotations];
  annotations.sort((a, b) => a.anchor.range.startLine - b.anchor.range.startLine);
  return annotations;
}

export function selectAnnotation(id: string | null) {
  state.selectedAnnotationId = id;
}

export async function addAnnotation(
  filePath: string, body: string, labels: string[],
  startLine: number, startColumn: number, endLine: number, endColumn: number
) {
  const annotation = await createAnnotation({
    filePath, body, labels, startLine, startColumn, endLine, endColumn,
  });
  if (state.sidecar) {
    state.sidecar.annotations = [...state.sidecar.annotations, annotation];
  }
  return annotation;
}

export async function editAnnotation(
  filePath: string, annotationId: string, body?: string, labels?: string[]
) {
  const updated = await updateAnnotation(filePath, annotationId, body, labels);
  if (state.sidecar) {
    state.sidecar.annotations = state.sidecar.annotations.map((a) =>
      a.id === annotationId ? updated : a
    );
  }
}

export async function removeAnnotation(filePath: string, annotationId: string) {
  await deleteAnnotation(filePath, annotationId);
  if (state.sidecar) {
    state.sidecar.annotations = state.sidecar.annotations.filter((a) => a.id !== annotationId);
  }
}
