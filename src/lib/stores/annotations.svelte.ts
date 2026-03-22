import { getAnnotations, createAnnotation, updateAnnotation, deleteAnnotation } from "$lib/tauri";
import type { Annotation, SidecarFile } from "$lib/types";

interface AnnotationsState {
  sidecar: SidecarFile | null;
  selectedAnnotationId: string | null;
}

let state = $state<AnnotationsState>({
  sidecar: null,
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
