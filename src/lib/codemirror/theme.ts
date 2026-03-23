import { EditorView } from "@codemirror/view";

export const redPenTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "14px",
    fontFamily: "var(--font-mono)",
    backgroundColor: "var(--surface-editor)",
  },
  ".cm-content": {
    padding: "8px 0",
    caretColor: "var(--accent)",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--accent)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--surface-editor)",
    borderRight: "1px solid var(--border-default)",
    color: "var(--text-muted)",
    fontSize: "12px",
    minWidth: "48px",
  },
  ".cm-lineNumbers .cm-gutterElement": {
    opacity: "0.5",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "transparent",
    color: "var(--text-secondary)",
    opacity: "1",
  },
  ".cm-activeLine": {
    backgroundColor: "color-mix(in srgb, var(--surface-highlight) 50%, transparent)",
  },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "var(--surface-selection) !important",
  },
  // Annotation highlight
  ".rp-annotation": {
    backgroundColor: "var(--accent-subtle)",
    borderBottom: "2px solid var(--accent-annotation-border)",
  },
  ".rp-annotation-orphaned": {
    backgroundColor: "rgba(217, 107, 95, 0.12)",
    borderBottom: "2px solid rgba(217, 107, 95, 0.5)",
  },
  // Gutter markers
  ".rp-annotation-gutter": {
    width: "14px",
  },
  ".rp-gutter-marker": {
    width: "3px",
    height: "100%",
    display: "block",
    backgroundColor: "var(--accent-annotation-border)",
    borderRadius: "1px",
    marginLeft: "5px",
    transition: "background-color 150ms",
  },
  ".rp-gutter-marker:hover": {
    backgroundColor: "var(--accent)",
  },
  ".rp-gutter-marker-orphaned": {
    backgroundColor: "rgba(217, 107, 95, 0.5)",
  },
  // Search highlights
  ".rp-search-match": {
    backgroundColor: "rgba(251, 191, 36, 0.2)",
    borderBottom: "2px solid rgba(251, 191, 36, 0.45)",
  },
  ".rp-search-current": {
    backgroundColor: "rgba(251, 191, 36, 0.45)",
    borderBottom: "2px solid rgba(251, 191, 36, 0.9)",
  },
});
