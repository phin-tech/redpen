import {
  Decoration,
  EditorView,
  gutter,
  GutterMarker,
} from "@codemirror/view";
import { StateField, StateEffect, RangeSetBuilder } from "@codemirror/state";
import type { Annotation, AnnotationKind } from "$lib/types";

// State effect to set annotations
export const setAnnotationsEffect = StateEffect.define<Annotation[]>();

// State field holding current annotations
export const annotationsField = StateField.define<Annotation[]>({
  create() {
    return [];
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setAnnotationsEffect)) {
        return effect.value;
      }
    }
    return value;
  },
});

/** Map annotation kind to a CSS class suffix */
function kindClass(kind: AnnotationKind): string {
  switch (kind) {
    case "explanation": return "rp-annotation-explanation";
    case "lineNote": return "rp-annotation-linenote";
    case "label": return "rp-annotation-label";
    default: return "rp-annotation";
  }
}

function gutterKindClass(kind: AnnotationKind): string {
  switch (kind) {
    case "explanation": return "rp-gutter-marker rp-gutter-marker-explanation";
    case "lineNote": return "rp-gutter-marker rp-gutter-marker-linenote";
    case "label": return "rp-gutter-marker rp-gutter-marker-label";
    default: return "rp-gutter-marker";
  }
}

// Mark decorations for annotation ranges
const annotationDecorations = EditorView.decorations.compute(
  [annotationsField],
  (state) => {
    const annotations = state.field(annotationsField);
    const builder = new RangeSetBuilder<Decoration>();

    // Sort by position for RangeSetBuilder (must be in order)
    const sorted = [...annotations].sort((a, b) => {
      const lineA = a.anchor.range.startLine;
      const lineB = b.anchor.range.startLine;
      if (lineA !== lineB) return lineA - lineB;
      return a.anchor.range.startColumn - b.anchor.range.startColumn;
    });

    for (const ann of sorted) {
      const { startLine, startColumn, endLine, endColumn } = ann.anchor.range;

      // Convert 1-based line to doc position
      if (startLine < 1 || startLine > state.doc.lines) continue;

      const startLineObj = state.doc.line(startLine);
      const from =
        startLineObj.from + Math.min(startColumn, startLineObj.length);

      let to: number;
      if (endLine <= state.doc.lines) {
        const endLineObj = state.doc.line(endLine);
        to = endLineObj.from + Math.min(endColumn, endLineObj.length);
      } else {
        to = from;
      }

      if (from > to) continue;

      const cssClass = ann.isOrphaned
        ? "rp-annotation-orphaned"
        : kindClass(ann.kind);

      if (from === to) {
        // Zero-width — mark the whole line instead
        const line = state.doc.line(startLine);
        builder.add(line.from, line.to, Decoration.mark({ class: cssClass }));
      } else {
        builder.add(from, to, Decoration.mark({ class: cssClass }));
      }
    }

    return builder.finish();
  }
);

// Gutter marker for annotated lines — vertical bar indicator
class AnnotationGutterMarker extends GutterMarker {
  constructor(private orphaned: boolean, private kind: AnnotationKind) {
    super();
  }

  toDOM() {
    const bar = document.createElement("span");
    bar.className = this.orphaned
      ? "rp-gutter-marker rp-gutter-marker-orphaned"
      : gutterKindClass(this.kind);
    return bar;
  }
}

const annotationGutter = gutter({
  class: "rp-annotation-gutter",
  markers: (view) => {
    const annotations = view.state.field(annotationsField);
    const builder = new RangeSetBuilder<GutterMarker>();

    // Group by line — orphaned takes priority, track dominant kind
    const lineMap = new Map<number, { orphaned: boolean; kind: AnnotationKind }>();
    for (const ann of annotations) {
      const line = ann.anchor.range.startLine;
      if (line < 1 || line > view.state.doc.lines) continue;
      const existing = lineMap.get(line);
      if (!existing) {
        lineMap.set(line, { orphaned: ann.isOrphaned, kind: ann.kind });
      } else {
        if (ann.isOrphaned) existing.orphaned = true;
      }
    }

    // Must add in order
    const sortedLines = [...lineMap.entries()].sort((a, b) => a[0] - b[0]);
    for (const [lineNum, { orphaned, kind }] of sortedLines) {
      const lineObj = view.state.doc.line(lineNum);
      builder.add(
        lineObj.from,
        lineObj.from,
        new AnnotationGutterMarker(orphaned, kind)
      );
    }

    return builder.finish();
  },
});

// Export all annotation extensions
export function annotationExtensions() {
  return [annotationsField, annotationDecorations, annotationGutter];
}
