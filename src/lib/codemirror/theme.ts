import { EditorView } from "@codemirror/view";

export const redPenTheme = EditorView.theme({
  "&": {
    height: "100%",
    fontSize: "13px",
    fontFamily: "'SF Mono', 'Fira Code', 'JetBrains Mono', monospace",
  },
  ".cm-content": {
    padding: "8px 0",
  },
  ".cm-gutters": {
    backgroundColor: "var(--bg-surface)",
    borderRight: "1px solid var(--border-color)",
    color: "var(--text-muted)",
  },
  ".cm-activeLineGutter": {
    backgroundColor: "var(--bg-highlight)",
  },
  // Annotation mark decorations
  ".rp-annotation-comment": {
    backgroundColor: "rgba(59, 130, 246, 0.15)",
    borderBottom: "2px solid rgba(59, 130, 246, 0.5)",
  },
  ".rp-annotation-lineNote": {
    backgroundColor: "rgba(234, 179, 8, 0.15)",
    borderBottom: "2px solid rgba(234, 179, 8, 0.5)",
  },
  ".rp-annotation-label": {
    backgroundColor: "rgba(168, 85, 247, 0.15)",
    borderBottom: "2px solid rgba(168, 85, 247, 0.5)",
  },
  ".rp-annotation-orphaned": {
    backgroundColor: "rgba(239, 68, 68, 0.15)",
    borderBottom: "2px solid rgba(239, 68, 68, 0.5)",
  },
  // Gutter dot
  ".rp-gutter-dot": {
    width: "8px",
    height: "8px",
    borderRadius: "50%",
    display: "inline-block",
    marginLeft: "4px",
  },
  ".rp-gutter-dot-comment": { backgroundColor: "#3b82f6" },
  ".rp-gutter-dot-lineNote": { backgroundColor: "#eab308" },
  ".rp-gutter-dot-label": { backgroundColor: "#a855f7" },
  ".rp-gutter-dot-orphaned": { backgroundColor: "#ef4444" },
});
