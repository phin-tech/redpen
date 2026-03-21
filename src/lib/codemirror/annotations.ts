import {
  Decoration,
  EditorView,
  gutter,
  GutterMarker,
} from "@codemirror/view";
import { StateField, StateEffect, RangeSetBuilder } from "@codemirror/state";
import type { Annotation } from "$lib/types";

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
        : `rp-annotation-${ann.kind}`;

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

// Gutter marker for annotated lines
class AnnotationGutterMarker extends GutterMarker {
  constructor(
    private kind: string,
    private orphaned: boolean
  ) {
    super();
  }

  toDOM() {
    const dot = document.createElement("span");
    dot.className = this.orphaned
      ? "rp-gutter-dot rp-gutter-dot-orphaned"
      : `rp-gutter-dot rp-gutter-dot-${this.kind}`;
    return dot;
  }
}

const annotationGutter = gutter({
  class: "rp-annotation-gutter",
  markers: (view) => {
    const annotations = view.state.field(annotationsField);
    const builder = new RangeSetBuilder<GutterMarker>();

    // Group by line — orphaned takes priority, then comment > lineNote > label
    const kindPriority: Record<string, number> = {
      comment: 3,
      lineNote: 2,
      label: 1,
    };
    const lineMap = new Map<number, { kind: string; orphaned: boolean }>();
    for (const ann of annotations) {
      const line = ann.anchor.range.startLine;
      if (line < 1 || line > view.state.doc.lines) continue;
      const existing = lineMap.get(line);
      if (!existing) {
        lineMap.set(line, { kind: ann.kind, orphaned: ann.isOrphaned });
      } else if (ann.isOrphaned && !existing.orphaned) {
        lineMap.set(line, { kind: ann.kind, orphaned: true });
      } else if (
        !ann.isOrphaned &&
        !existing.orphaned &&
        (kindPriority[ann.kind] ?? 0) > (kindPriority[existing.kind] ?? 0)
      ) {
        lineMap.set(line, { kind: ann.kind, orphaned: false });
      }
    }

    // Must add in order
    const sortedLines = [...lineMap.entries()].sort((a, b) => a[0] - b[0]);
    for (const [lineNum, { kind, orphaned }] of sortedLines) {
      const lineObj = view.state.doc.line(lineNum);
      builder.add(
        lineObj.from,
        lineObj.from,
        new AnnotationGutterMarker(kind, orphaned)
      );
    }

    return builder.finish();
  },
});

// Export all annotation extensions
export function annotationExtensions() {
  return [annotationsField, annotationDecorations, annotationGutter];
}
