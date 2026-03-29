import { EditorSelection, type SelectionRange } from "@codemirror/state";
import { EditorView } from "@codemirror/view";

type VisualMode = "char" | "line" | null;

interface VimishState {
  mode: VisualMode;
  anchor: number;
  head: number;
  preferredColumn: number;
}

const stateByView = new WeakMap<EditorView, VimishState>();

function getInitialState(view: EditorView): VimishState {
  const head = view.state.selection.main.head;
  const line = view.state.doc.lineAt(head);
  return {
    mode: null,
    anchor: head,
    head,
    preferredColumn: head - line.from,
  };
}

function getState(view: EditorView): VimishState {
  const existing = stateByView.get(view);
  if (existing) {
    if (existing.mode === null) {
      const head = view.state.selection.main.head;
      const line = view.state.doc.lineAt(head);
      existing.anchor = head;
      existing.head = head;
      existing.preferredColumn = head - line.from;
    }
    return existing;
  }

  const initial = getInitialState(view);
  stateByView.set(view, initial);
  return initial;
}

function dispatchSelection(view: EditorView, selection: SelectionRange) {
  view.dispatch({
    selection,
    effects: EditorView.scrollIntoView(selection.head, { y: "center" }),
  });
}

function applySelection(view: EditorView, state: VimishState) {
  if (state.mode === null) {
    dispatchSelection(view, EditorSelection.cursor(state.head));
    return;
  }

  if (state.mode === "char") {
    dispatchSelection(view, EditorSelection.range(state.anchor, state.head));
    return;
  }

  const anchorLine = view.state.doc.lineAt(state.anchor);
  const headLine = view.state.doc.lineAt(state.head);
  const fromLineNumber = Math.min(anchorLine.number, headLine.number);
  const toLineNumber = Math.max(anchorLine.number, headLine.number);
  const from = view.state.doc.line(fromLineNumber).from;
  const to = view.state.doc.line(toLineNumber).to;
  dispatchSelection(view, EditorSelection.range(from, to));
}

function moveToLine(view: EditorView, targetLineNumber: number) {
  const state = getState(view);
  const doc = view.state.doc;
  const currentHead = state.head;
  const currentLine = doc.lineAt(currentHead);
  const targetLine = doc.line(Math.max(1, Math.min(targetLineNumber, doc.lines)));
  const column = state.preferredColumn ?? (currentHead - currentLine.from);
  state.head = targetLine.from + Math.min(column, targetLine.length);
  state.preferredColumn = column;
  applySelection(view, state);
}

export function moveCursorLine(view: EditorView, dir: 1 | -1) {
  const state = getState(view);
  const currentLine = view.state.doc.lineAt(state.head);
  moveToLine(view, currentLine.number + dir);
}

export function jumpToBoundary(view: EditorView, boundary: "top" | "bottom") {
  moveToLine(view, boundary === "top" ? 1 : view.state.doc.lines);
}

export function toggleVisualSelection(view: EditorView, mode: Exclude<VisualMode, null>) {
  const state = getState(view);
  const currentHead = state.mode === null ? view.state.selection.main.head : state.head;

  if (state.mode === mode) {
    state.mode = null;
    state.anchor = currentHead;
    state.head = currentHead;
    const line = view.state.doc.lineAt(currentHead);
    state.preferredColumn = currentHead - line.from;
    applySelection(view, state);
    return;
  }

  state.mode = mode;
  state.anchor = currentHead;
  state.head = currentHead;
  const line = view.state.doc.lineAt(currentHead);
  state.preferredColumn = currentHead - line.from;
  applySelection(view, state);
}

export function clearVisualSelection(view: EditorView) {
  const state = getState(view);
  if (state.mode === null) return;
  state.mode = null;
  applySelection(view, state);
}

export function hasVisualSelection(view: EditorView): boolean {
  return getState(view).mode !== null;
}
