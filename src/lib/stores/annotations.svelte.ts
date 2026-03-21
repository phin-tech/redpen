import { getAnnotations, createAnnotation, updateAnnotation, deleteAnnotation } from "$lib/tauri";
import type { Annotation, AnnotationFilter, AnnotationKind, SidecarFile } from "$lib/types";

interface AnnotationsState {
  sidecar: SidecarFile | null;
  filter: AnnotationFilter;
  selectedAnnotationId: string | null;
}

let state = $state<AnnotationsState>({
  sidecar: null,
  filter: "all",
  selectedAnnotationId: null,
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

export function filteredAnnotations(): Annotation[] {
  if (!state.sidecar) return [];
  let annotations = [...state.sidecar.annotations];
  if (state.filter !== "all") {
    annotations = annotations.filter((a) => a.kind === state.filter);
  }
  annotations.sort((a, b) => a.anchor.range.startLine - b.anchor.range.startLine);
  return annotations;
}

export function setFilter(filter: AnnotationFilter) {
  state.filter = filter;
}

export function selectAnnotation(id: string | null) {
  state.selectedAnnotationId = id;
}

export async function addAnnotation(
  filePath: string, kind: AnnotationKind, body: string, labels: string[],
  startLine: number, startColumn: number, endLine: number, endColumn: number
) {
  const annotation = await createAnnotation({
    filePath, kind, body, labels, startLine, startColumn, endLine, endColumn,
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
