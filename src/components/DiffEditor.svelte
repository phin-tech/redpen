<script lang="ts">
    import { onDestroy } from "svelte";
    import { EditorView, lineNumbers, highlightActiveLine, drawSelection } from "@codemirror/view";
    import { EditorState, type Extension } from "@codemirror/state";
    import { syntaxHighlighting, defaultHighlightStyle } from "@codemirror/language";
    import { oneDark } from "@codemirror/theme-one-dark";
    import { getLanguageForExtension } from "$lib/codemirror/languages";
    import { redPenTheme } from "$lib/codemirror/theme";
    import {
        moveCursorLine,
        jumpToBoundary,
        toggleVisualSelection,
        clearVisualSelection,
        hasVisualSelection,
    } from "$lib/codemirror/vimish";

    interface Props {
        content: string;
        fileExtension: string;
        diffExtensions?: Extension;
        showLineNumbers?: boolean;
        onSelectionChange?: (fromLine: number, fromCol: number, toLine: number, toCol: number) => void;
    }

    let { content, fileExtension, diffExtensions, showLineNumbers = true, onSelectionChange }: Props = $props();

    let container: HTMLDivElement;
    // EditorView instance is imperative lifecycle state, not render state.
    // Keep it non-reactive so the recreation effect doesn't subscribe to and
    // retrigger itself on `view = null` / `view = new EditorView(...)`.
    let view: EditorView | null = null;

    export function getView(): EditorView | null {
        return view;
    }

    export function moveCursorByLine(dir: 1 | -1) {
        if (view) moveCursorLine(view, dir);
    }

    export function jumpCursorToBoundary(boundary: "top" | "bottom") {
        if (view) jumpToBoundary(view, boundary);
    }

    export function toggleVisualMode(mode: "char" | "line") {
        if (view) toggleVisualSelection(view, mode);
    }

    export function clearVisualMode() {
        if (view) clearVisualSelection(view);
    }

    export function hasVisualMode(): boolean {
        return view ? hasVisualSelection(view) : false;
    }

    onDestroy(() => {
        view?.destroy();
        view = null;
    });

    // When content or diffExtensions change (e.g., switching files in diff mode),
    // destroy and recreate the view. Updating just the doc is insufficient because
    // diff extensions (decorations, gutters, ghost lines) are tightly coupled to
    // the specific diff result and cannot be reconfigured in-place.
    $effect(() => {
        // Track these props to trigger recreation
        const _content = content;
        const _ext = diffExtensions;
        const _fileExt = fileExtension;

        if (!container) return;

        // Destroy previous view
        if (view) {
            view.destroy();
            view = null;
        }

        const extensions: Extension[] = [
            EditorState.readOnly.of(true),
            EditorView.editable.of(false),
            highlightActiveLine(),
            drawSelection(),
            syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
            oneDark,
            redPenTheme,
        ];

        if (showLineNumbers) {
            extensions.push(lineNumbers());
        }

        const lang = getLanguageForExtension(_fileExt);
        if (lang) extensions.push(lang);

        if (_ext) {
            extensions.push(_ext);
        }

        if (onSelectionChange) {
            extensions.push(EditorView.updateListener.of((update) => {
                if (update.selectionSet) {
                    const sel = update.state.selection.main;
                    const from = update.state.doc.lineAt(sel.from);
                    const to = update.state.doc.lineAt(sel.to);
                    onSelectionChange!(from.number, sel.from - from.from + 1, to.number, sel.to - to.from + 1);
                }
            }));
        }

        view = new EditorView({
            state: EditorState.create({ doc: _content, extensions }),
            parent: container,
        });
    });
</script>

<div bind:this={container} class="diff-editor"></div>

<style>
    .diff-editor {
        height: 100%;
        overflow: auto;
    }
    .diff-editor :global(.cm-editor) {
        height: 100%;
    }
    .diff-editor :global(.cm-diff-old-gutter .cm-gutterElement),
    .diff-editor :global(.cm-diff-new-gutter .cm-gutterElement) {
        padding: 0 8px;
        min-width: 32px;
        text-align: right;
        font-size: inherit;
    }
</style>
