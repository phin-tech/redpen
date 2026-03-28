import {
  Decoration,
  EditorView,
  WidgetType,
} from "@codemirror/view";
import {
  StateField,
  StateEffect,
  Facet,
  RangeSetBuilder,
} from "@codemirror/state";
import { mount, unmount } from "svelte";
import { annotationsField } from "./annotations";
import AnnotationBubble from "../../components/AnnotationBubble.svelte";
import type { Annotation } from "$lib/types";

// --- State effects ---

export const setBubblesEnabledEffect = StateEffect.define<boolean>();
export const toggleBubbleExpansionEffect = StateEffect.define<number>(); // line number

// --- Callback facet ---

interface BubbleCallbacks {
  onSelect: (id: string) => void;
  onDelete: (id: string) => void;
}

export const bubbleCallbacksFacet = Facet.define<BubbleCallbacks, BubbleCallbacks>({
  combine(values) {
    return values[0] ?? { onSelect: () => {}, onDelete: () => {} };
  },
});

// --- State fields ---

export const bubblesEnabledField = StateField.define<boolean>({
  create() {
    return true;
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setBubblesEnabledEffect)) {
        return effect.value;
      }
    }
    return value;
  },
});

const bubbleExpansionField = StateField.define<Set<number>>({
  create() {
    return new Set();
  },
  update(value, tr) {
    for (const effect of tr.effects) {
      if (effect.is(toggleBubbleExpansionEffect)) {
        const next = new Set(value);
        if (next.has(effect.value)) {
          next.delete(effect.value);
        } else {
          next.add(effect.value);
        }
        return next;
      }
    }
    return value;
  },
});

// --- Widget ---

/** Group annotations by their root's startLine */
function groupByLine(annotations: Annotation[]): Map<number, Annotation[]> {
  const roots = annotations.filter((a) => !a.replyTo);
  const replyMap = new Map<string, Annotation[]>();
  for (const a of annotations) {
    if (a.replyTo) {
      const group = replyMap.get(a.replyTo) || [];
      group.push(a);
      replyMap.set(a.replyTo, group);
    }
  }

  const lineMap = new Map<number, Annotation[]>();
  for (const root of roots) {
    const line = root.anchor.range.startLine;
    const group = lineMap.get(line) || [];
    group.push(root);
    const replies = replyMap.get(root.id) || [];
    replies.sort((a, b) => (a.createdAt ?? "").localeCompare(b.createdAt ?? ""));
    group.push(...replies);
    lineMap.set(line, group);
  }

  return lineMap;
}

// Module-level mutable ref so widgets can dispatch effects back to the view.
// Captured by an updateListener below.
let activeViewRef: { view: EditorView | null } = { view: null };

class AnnotationBubbleWidget extends WidgetType {
  private svelteComponent: Record<string, unknown> | null = null;

  constructor(
    readonly annotations: Annotation[],
    readonly lineNum: number,
    readonly expanded: boolean,
    readonly callbacks: BubbleCallbacks,
  ) {
    super();
  }

  eq(other: AnnotationBubbleWidget) {
    if (this.annotations.length !== other.annotations.length) return false;
    if (this.expanded !== other.expanded) return false;
    for (let i = 0; i < this.annotations.length; i++) {
      const a = this.annotations[i];
      const b = other.annotations[i];
      if (a.id !== b.id || a.body !== b.body || a.isOrphaned !== b.isOrphaned) return false;
    }
    return true;
  }

  toDOM(): HTMLElement {
    const wrapper = document.createElement("div");
    wrapper.className = "rp-bubble-widget";

    const lineNum = this.lineNum;
    const callbacks = this.callbacks;

    this.svelteComponent = mount(AnnotationBubble, {
      target: wrapper,
      props: {
        annotations: this.annotations,
        expanded: this.expanded,
        onToggle: () => {
          activeViewRef.view?.dispatch({
            effects: toggleBubbleExpansionEffect.of(lineNum),
          });
        },
        onSelect: callbacks.onSelect,
        onDelete: callbacks.onDelete,
      },
    });

    return wrapper;
  }

  destroy(_dom: HTMLElement) {
    if (this.svelteComponent) {
      unmount(this.svelteComponent);
      this.svelteComponent = null;
    }
  }

  get estimatedHeight() {
    return this.expanded ? 80 : 32;
  }

  ignoreEvent() {
    return true; // Let events pass through to Svelte
  }
}

// --- View capture ---

const bubbleViewCapture = EditorView.updateListener.of((update) => {
  activeViewRef.view = update.view;
});

// --- Decoration provider ---

const bubbleDecorations = EditorView.decorations.compute(
  [annotationsField, bubblesEnabledField, bubbleExpansionField],
  (state) => {
    const enabled = state.field(bubblesEnabledField);
    if (!enabled) return Decoration.none;

    const annotations = state.field(annotationsField);
    if (annotations.length === 0) return Decoration.none;

    const expandedLines = state.field(bubbleExpansionField);
    const lineGroups = groupByLine(annotations);
    const callbacks = state.facet(bubbleCallbacksFacet);

    const builder = new RangeSetBuilder<Decoration>();
    const sortedLines = [...lineGroups.entries()].sort((a, b) => a[0] - b[0]);

    for (const [lineNum, group] of sortedLines) {
      if (lineNum < 1 || lineNum > state.doc.lines) continue;
      const lineObj = state.doc.line(lineNum);

      const widget = new AnnotationBubbleWidget(
        group,
        lineNum,
        expandedLines.has(lineNum),
        callbacks,
      );

      builder.add(
        lineObj.to,
        lineObj.to,
        Decoration.widget({ widget, block: true, side: 1 }),
      );
    }

    return builder.finish();
  },
);

// --- Export ---

export function bubbleExtensions() {
  return [
    bubblesEnabledField,
    bubbleExpansionField,
    bubbleViewCapture,
    bubbleDecorations,
  ];
}
