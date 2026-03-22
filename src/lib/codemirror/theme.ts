import { EditorView } from "@codemirror/view";

export const redPenTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "14px",
    fontFamily: "var(--font-mono)",
    backgroundColor: "var(--bg-editor)",
  },
  ".cm-content": {
    padding: "8px 0",
    caretColor: "var(--accent)",
  },
  ".cm-cursor": {
    borderLeftColor: "var(--accent)",
  },
  ".cm-gutters": {
    backgroundColor: "var(--bg-editor)",
    borderRight: "1px solid var(--border-color)",
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
    backgroundColor: "rgba(42, 36, 32, 0.5)",
  },
  ".cm-selectionBackground, ::selection": {
    backgroundColor: "var(--bg-selection) !important",
  },
  // Annotation highlight
  ".rp-annotation": {
    backgroundColor: "rgba(227, 154, 45, 0.12)",
    borderBottom: "2px solid rgba(227, 154, 45, 0.5)",
  },
  ".rp-annotation-orphaned": {
    backgroundColor: "rgba(217, 107, 95, 0.12)",
    borderBottom: "2px solid rgba(217, 107, 95, 0.5)",
  },
  // Gutter dots
  ".rp-gutter-dot": {
    width: "6px",
    height: "6px",
    borderRadius: "50%",
    display: "inline-block",
    marginLeft: "6px",
    backgroundColor: "#E39A2D",
  },
  ".rp-gutter-dot-orphaned": { backgroundColor: "#D96B5F" },
});
