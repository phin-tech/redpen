import {
    Decoration,
    type DecorationSet,
    EditorView,
    ViewPlugin,
    type ViewUpdate,
    WidgetType,
    GutterMarker,
    gutter,
} from "@codemirror/view";
import { RangeSetBuilder, StateEffect, StateField, type Extension } from "@codemirror/state";
import type { DiffResult } from "$lib/types";

const addedLine = Decoration.line({ class: "rp-diff-added" });
const removedLine = Decoration.line({ class: "rp-diff-removed" });

class GhostLineWidget extends WidgetType {
    constructor(readonly count: number) { super(); }
    toDOM(view: EditorView) {
        const div = document.createElement("div");
        div.className = "rp-diff-ghost";
        const lineHeight = view.defaultLineHeight;
        div.style.height = `${this.count * lineHeight}px`;
        return div;
    }
    eq(other: GhostLineWidget) { return this.count === other.count; }
    ignoreEvent() { return true; }
}

class DiffLineNumber extends GutterMarker {
    constructor(readonly num: number | null) { super(); }
    toDOM() {
        const span = document.createElement("span");
        span.textContent = this.num != null ? String(this.num) : "";
        return span;
    }
}

// --- Diff folding: collapse unchanged regions ---

const unfoldEffect = StateEffect.define<{ from: number; to: number }>();

const unfoldedRangesField = StateField.define<Array<{ from: number; to: number }>>({
    create() { return []; },
    update(value, tr) {
        for (const effect of tr.effects) {
            if (effect.is(unfoldEffect)) {
                return [...value, effect.value];
            }
        }
        return value;
    },
});

class DiffFoldWidget extends WidgetType {
    constructor(readonly lineCount: number, readonly foldFrom: number, readonly foldTo: number) {
        super();
    }
    toDOM(view: EditorView) {
        const div = document.createElement("div");
        div.className = "rp-diff-fold";
        div.textContent = `⋯ ${this.lineCount} unchanged lines ⋯`;
        div.addEventListener("click", () => {
            view.dispatch({ effects: unfoldEffect.of({ from: this.foldFrom, to: this.foldTo }) });
        });
        return div;
    }
    eq(other: DiffFoldWidget) {
        return this.foldFrom === other.foldFrom && this.foldTo === other.foldTo;
    }
    ignoreEvent() { return false; }
}

/** Minimum gap size (in lines) worth collapsing */
const MIN_FOLD_LINES = 4;

function computeFoldRanges(diffResult: DiffResult, totalLines: number): Array<{ fromLine: number; toLine: number }> {
    const ranges: Array<{ fromLine: number; toLine: number }> = [];
    // Collect the visible line ranges from hunks (using newLine numbers)
    const visibleRanges: Array<{ start: number; end: number }> = [];
    for (const hunk of diffResult.hunks) {
        if (hunk.newStart > 0 && hunk.newCount > 0) {
            visibleRanges.push({ start: hunk.newStart, end: hunk.newStart + hunk.newCount - 1 });
        }
    }

    if (visibleRanges.length === 0) return [];

    // Gap before first hunk
    if (visibleRanges[0].start > 1) {
        const gapEnd = visibleRanges[0].start - 1;
        if (gapEnd >= MIN_FOLD_LINES) {
            ranges.push({ fromLine: 1, toLine: gapEnd });
        }
    }

    // Gaps between hunks
    for (let i = 0; i < visibleRanges.length - 1; i++) {
        const gapStart = visibleRanges[i].end + 1;
        const gapEnd = visibleRanges[i + 1].start - 1;
        if (gapEnd - gapStart + 1 >= MIN_FOLD_LINES) {
            ranges.push({ fromLine: gapStart, toLine: gapEnd });
        }
    }

    // Gap after last hunk
    const lastEnd = visibleRanges[visibleRanges.length - 1].end;
    if (lastEnd < totalLines) {
        const gapSize = totalLines - lastEnd;
        if (gapSize >= MIN_FOLD_LINES) {
            ranges.push({ fromLine: lastEnd + 1, toLine: totalLines });
        }
    }

    return ranges;
}

export function highlightsModeExtensions(diffResult: DiffResult): Extension {
    // Use pre-sorted arrays from Rust — no JS sorting needed
    const sorted = diffResult.insertedLines;

    const highlightPlugin = ViewPlugin.fromClass(
        class {
            decorations: DecorationSet;
            constructor(view: EditorView) {
                this.decorations = this.buildDecorations(view);
            }
            update(update: ViewUpdate) {
                if (update.docChanged || update.viewportChanged) {
                    this.decorations = this.buildDecorations(update.view);
                }
            }
            buildDecorations(view: EditorView): DecorationSet {
                const builder = new RangeSetBuilder<Decoration>();
                for (const lineNum of sorted) {
                    if (lineNum <= view.state.doc.lines) {
                        const line = view.state.doc.line(lineNum);
                        builder.add(line.from, line.from, addedLine);
                    }
                }
                return builder.finish();
            }
        },
        { decorations: (v) => v.decorations }
    );

    const foldPlugin = ViewPlugin.fromClass(
        class {
            decorations: DecorationSet;
            constructor(view: EditorView) {
                this.decorations = this.buildFolds(view);
            }
            update(update: ViewUpdate) {
                if (update.docChanged || update.startState.field(unfoldedRangesField) !== update.state.field(unfoldedRangesField)) {
                    this.decorations = this.buildFolds(update.view);
                }
            }
            buildFolds(view: EditorView): DecorationSet {
                const totalLines = view.state.doc.lines;
                const foldRanges = computeFoldRanges(diffResult, totalLines);
                const unfolded = view.state.field(unfoldedRangesField);
                const builder = new RangeSetBuilder<Decoration>();

                for (const range of foldRanges) {
                    // Skip if user has unfolded this range
                    if (unfolded.some(u => u.from === range.fromLine && u.to === range.toLine)) continue;

                    if (range.fromLine > totalLines) continue;
                    const clampedTo = Math.min(range.toLine, totalLines);
                    const fromPos = view.state.doc.line(range.fromLine).from;
                    const toPos = view.state.doc.line(clampedTo).to;
                    const lineCount = clampedTo - range.fromLine + 1;

                    builder.add(fromPos, toPos, Decoration.replace({
                        widget: new DiffFoldWidget(lineCount, range.fromLine, range.toLine),
                        block: true,
                    }));
                }

                return builder.finish();
            }
        },
        { decorations: (v) => v.decorations, provide: () => [] }
    );

    return [unfoldedRangesField, highlightPlugin, foldPlugin];
}

export interface UnifiedDocResult {
    syntheticDoc: string;
    lineMap: Map<number, number>;
    extensions: Extension;
}

export function buildUnifiedDocument(diffResult: DiffResult): UnifiedDocResult {
    const lines: string[] = [];
    const lineMap = new Map<number, number>();
    const lineTypes: Array<"equal" | "insert" | "delete"> = [];
    const oldLineNums: Array<number | null> = [];
    const newLineNums: Array<number | null> = [];

    let syntheticLine = 0;
    for (const hunk of diffResult.hunks) {
        for (const change of hunk.changes) {
            syntheticLine++;
            lines.push(change.content.replace(/\n$/, ""));
            lineTypes.push(change.kind);
            oldLineNums.push(change.oldLine);
            newLineNums.push(change.newLine);
            if (change.kind !== "delete" && change.newLine != null) {
                lineMap.set(syntheticLine, change.newLine);
            }
        }
    }

    const syntheticDoc = lines.join("\n");

    const highlightPlugin = ViewPlugin.fromClass(
        class {
            decorations: DecorationSet;
            constructor(view: EditorView) {
                this.decorations = this.buildDecorations(view);
            }
            update(update: ViewUpdate) {
                if (update.docChanged) {
                    this.decorations = this.buildDecorations(update.view);
                }
            }
            buildDecorations(view: EditorView): DecorationSet {
                const builder = new RangeSetBuilder<Decoration>();
                for (let i = 0; i < lineTypes.length; i++) {
                    const lineNum = i + 1;
                    if (lineNum <= view.state.doc.lines) {
                        const line = view.state.doc.line(lineNum);
                        if (lineTypes[i] === "insert") {
                            builder.add(line.from, line.from, addedLine);
                        } else if (lineTypes[i] === "delete") {
                            builder.add(line.from, line.from, removedLine);
                        }
                    }
                }
                return builder.finish();
            }
        },
        { decorations: (v) => v.decorations }
    );

    const oldGutter = gutter({
        class: "cm-diff-old-gutter",
        lineMarker: (_view, line) => {
            const lineNum = _view.state.doc.lineAt(line.from).number;
            const idx = lineNum - 1;
            if (idx < oldLineNums.length) {
                return new DiffLineNumber(oldLineNums[idx]);
            }
            return null;
        },
    });

    const newGutter = gutter({
        class: "cm-diff-new-gutter",
        lineMarker: (_view, line) => {
            const lineNum = _view.state.doc.lineAt(line.from).number;
            const idx = lineNum - 1;
            if (idx < newLineNums.length) {
                return new DiffLineNumber(newLineNums[idx]);
            }
            return null;
        },
    });

    return {
        syntheticDoc,
        lineMap,
        extensions: [oldGutter, newGutter, highlightPlugin],
    };
}

export interface SplitDecorationResult {
    oldExtensions: Extension;
    newExtensions: Extension;
}

export function buildSplitDecorations(diffResult: DiffResult): SplitDecorationResult {
    // Use pre-sorted arrays from Rust for line highlighting
    const sortedDeleted = diffResult.deletedLines;
    const sortedInserted = diffResult.insertedLines;

    // Ghost lines still need per-hunk computation
    const ghostsOld = new Map<number, number>();
    const ghostsNew = new Map<number, number>();

    for (const hunk of diffResult.hunks) {
        let deletes = 0;
        let inserts = 0;
        let lastDeleteLine = 0;
        let lastInsertLine = 0;

        for (const change of hunk.changes) {
            if (change.kind === "delete" && change.oldLine != null) {
                deletes++;
                lastDeleteLine = change.oldLine;
            } else if (change.kind === "insert" && change.newLine != null) {
                inserts++;
                lastInsertLine = change.newLine;
            }
        }

        if (inserts > deletes && lastDeleteLine > 0) {
            ghostsOld.set(lastDeleteLine, inserts - deletes);
        } else if (deletes > inserts && lastInsertLine > 0) {
            ghostsNew.set(lastInsertLine, deletes - inserts);
        }
    }

    const oldExtensions = ViewPlugin.fromClass(
        class {
            decorations: DecorationSet;
            constructor(view: EditorView) { this.decorations = this.build(view); }
            update(u: ViewUpdate) { if (u.docChanged) this.decorations = this.build(u.view); }
            build(view: EditorView): DecorationSet {
                const builder = new RangeSetBuilder<Decoration>();
                for (const lineNum of sortedDeleted) {
                    if (lineNum <= view.state.doc.lines) {
                        const line = view.state.doc.line(lineNum);
                        builder.add(line.from, line.from, removedLine);
                    }
                }
                for (const [afterLine, count] of Array.from(ghostsOld.entries()).sort((a, b) => a[0] - b[0])) {
                    if (afterLine <= view.state.doc.lines) {
                        const line = view.state.doc.line(afterLine);
                        builder.add(line.to, line.to, Decoration.widget({
                            widget: new GhostLineWidget(count),
                            side: 1,
                            block: true,
                        }));
                    }
                }
                return builder.finish();
            }
        },
        { decorations: (v) => v.decorations }
    );

    const newExtensions = ViewPlugin.fromClass(
        class {
            decorations: DecorationSet;
            constructor(view: EditorView) { this.decorations = this.build(view); }
            update(u: ViewUpdate) { if (u.docChanged) this.decorations = this.build(u.view); }
            build(view: EditorView): DecorationSet {
                const builder = new RangeSetBuilder<Decoration>();
                for (const lineNum of sortedInserted) {
                    if (lineNum <= view.state.doc.lines) {
                        const line = view.state.doc.line(lineNum);
                        builder.add(line.from, line.from, addedLine);
                    }
                }
                for (const [afterLine, count] of Array.from(ghostsNew.entries()).sort((a, b) => a[0] - b[0])) {
                    if (afterLine <= view.state.doc.lines) {
                        const line = view.state.doc.line(afterLine);
                        builder.add(line.to, line.to, Decoration.widget({
                            widget: new GhostLineWidget(count),
                            side: 1,
                            block: true,
                        }));
                    }
                }
                return builder.finish();
            }
        },
        { decorations: (v) => v.decorations }
    );

    return { oldExtensions, newExtensions };
}

export function scrollSync(leftView: EditorView, rightView: EditorView): () => void {
    let syncing = false;

    function syncScroll(source: EditorView, target: EditorView) {
        if (syncing) return;
        syncing = true;

        const sourceDOM = source.scrollDOM;
        const targetDOM = target.scrollDOM;
        const sourceMax = sourceDOM.scrollHeight - sourceDOM.clientHeight;
        const targetMax = targetDOM.scrollHeight - targetDOM.clientHeight;

        if (sourceMax > 0 && targetMax > 0) {
            const ratio = sourceDOM.scrollTop / sourceMax;
            targetDOM.scrollTop = ratio * targetMax;
        }

        syncing = false;
    }

    const onLeftScroll = () => syncScroll(leftView, rightView);
    const onRightScroll = () => syncScroll(rightView, leftView);

    leftView.scrollDOM.addEventListener("scroll", onLeftScroll);
    rightView.scrollDOM.addEventListener("scroll", onRightScroll);

    return () => {
        leftView.scrollDOM.removeEventListener("scroll", onLeftScroll);
        rightView.scrollDOM.removeEventListener("scroll", onRightScroll);
    };
}
