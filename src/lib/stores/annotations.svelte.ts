import { getAnnotations, createAnnotation, updateAnnotation, deleteAnnotation, clearAnnotations as clearAnnotationsApi, getAllAnnotations } from "$lib/tauri";
import type { Choice } from "$lib/types";
import type { Annotation, FileAnnotations, SidecarFile } from "$lib/types";

type AnnotationFilter = "all" | "comment" | "lineNote" | "label" | "explanation";
type SidebarView = "file" | "project";

type AnnotationKind = "comment" | "lineNote" | "label" | "explanation";
const ALL_KINDS: AnnotationKind[] = ["comment", "lineNote", "label", "explanation"];

interface AnnotationsState {
  sidecar: SidecarFile | null;
  selectedAnnotationId: string | null;
  filter: AnnotationFilter;
  sidebarView: SidebarView;
  projectAnnotations: FileAnnotations[];
  projectAnnotationsLoading: boolean;
  bubblesEnabled: boolean;
  bubbleKindFilter: Set<AnnotationKind>;
}

let state = $state<AnnotationsState>({
  sidecar: null,
  selectedAnnotationId: null,
  filter: "all",
  sidebarView: "file",
  projectAnnotations: [],
  projectAnnotationsLoading: false,
  bubblesEnabled: true,
  bubbleKindFilter: new Set<AnnotationKind>(ALL_KINDS),
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
  const all = [...state.sidecar.annotations];
  const roots = all.filter((a) => !a.replyTo);
  const replies = all.filter((a) => a.replyTo);

  // Group replies by parent ID, sorted by createdAt
  const replyMap = new Map<string, Annotation[]>();
  for (const r of replies) {
    const group = replyMap.get(r.replyTo!) || [];
    group.push(r);
    replyMap.set(r.replyTo!, group);
  }
  for (const group of replyMap.values()) {
    group.sort((a, b) => (a.createdAt ?? "").localeCompare(b.createdAt ?? ""));
  }

  // Sort roots by line, then interleave replies after their parent
  roots.sort((a, b) => a.anchor.range.startLine - b.anchor.range.startLine);
  const result: Annotation[] = [];
  for (const root of roots) {
    result.push(root);
    result.push(...(replyMap.get(root.id) || []));
  }

  // Orphan replies (parent deleted) go at end
  const rootIds = new Set(roots.map((r) => r.id));
  for (const r of replies) {
    if (!rootIds.has(r.replyTo!)) {
      result.push(r);
    }
  }

  return result;
}

export function selectAnnotation(id: string | null) {
  state.selectedAnnotationId = id;
}

export async function addAnnotation(
  filePath: string, body: string, labels: string[],
  startLine: number, startColumn: number, endLine: number, endColumn: number,
  kind?: import("$lib/types").AnnotationKind,
  replyTo?: string,
) {
  const annotation = await createAnnotation({
    filePath, body, labels, kind, startLine, startColumn, endLine, endColumn, replyTo,
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

export async function updateChoices(filePath: string, annotationId: string, choices: Choice[]) {
  const updated = await updateAnnotation(filePath, annotationId, undefined, undefined, choices);
  if (state.sidecar) {
    state.sidecar.annotations = state.sidecar.annotations.map((a) =>
      a.id === annotationId ? updated : a
    );
  }
}

export async function resolveAnnotation(filePath: string, annotationId: string, resolved: boolean) {
  const updated = await updateAnnotation(filePath, annotationId, undefined, undefined, undefined, resolved);
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

export function getBubblesEnabled() {
  return state.bubblesEnabled;
}

export function toggleBubbles() {
  state.bubblesEnabled = !state.bubblesEnabled;
}

export function getBubbleKindFilter(): Set<AnnotationKind> {
  return state.bubbleKindFilter;
}

export function toggleBubbleKind(kind: AnnotationKind) {
  const next = new Set(state.bubbleKindFilter);
  if (next.has(kind)) {
    // Don't allow disabling all kinds
    if (next.size > 1) next.delete(kind);
  } else {
    next.add(kind);
  }
  state.bubbleKindFilter = next;
}

export { ALL_KINDS, type AnnotationKind as BubbleAnnotationKind };

export async function clearAllAnnotations(filePath: string) {
  await clearAnnotationsApi(filePath);
  if (state.sidecar) {
    state.sidecar.annotations = [];
  }
}
