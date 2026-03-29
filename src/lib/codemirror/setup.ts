import {
  EditorView,
  lineNumbers,
  highlightActiveLine,
  drawSelection,
  type ViewUpdate,
} from "@codemirror/view";
import { EditorState } from "@codemirror/state";
import { oneDark } from "@codemirror/theme-one-dark";
import {
  syntaxHighlighting,
  defaultHighlightStyle,
} from "@codemirror/language";
import { redPenTheme } from "./theme";
import { annotationExtensions } from "./annotations";
import { bubbleExtensions, bubbleCallbacksFacet } from "./bubbles";
import { searchExtension } from "./search";
import { getLanguageForExtension } from "./languages";

export interface EditorConfig {
  content: string;
  extension: string;
  parent: HTMLElement;
  onSelectionChange?: (
    from: number,
    to: number,
    fromLine: number,
    fromCol: number,
    toLine: number,
    toCol: number
  ) => void;
  bubbleCallbacks?: {
    onSelect: (id: string) => void;
    onDelete: (id: string) => void;
    onChoiceToggle: (annotationId: string, choiceIndex: number) => void;
  };
}

export function createEditor(config: EditorConfig): EditorView {
  const extensions = [
    EditorState.readOnly.of(true),
    EditorView.editable.of(false),
    lineNumbers(),
    highlightActiveLine(),
    drawSelection(),
    syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
    oneDark,
    redPenTheme,
    ...annotationExtensions(),
    ...bubbleExtensions(),
    ...searchExtension(),
  ];

  if (config.bubbleCallbacks) {
    extensions.push(
      bubbleCallbacksFacet.of(config.bubbleCallbacks),
    );
  }

  // Add language support if available
  const lang = getLanguageForExtension(config.extension);
  if (lang) {
    extensions.push(lang);
  }

  // Track selection changes for annotation creation
  if (config.onSelectionChange) {
    const onSelectionChange = config.onSelectionChange;
    extensions.push(
      EditorView.updateListener.of((update: ViewUpdate) => {
        if (update.selectionSet) {
          const sel = update.state.selection.main;
          if (sel.from !== sel.to) {
            const fromLine = update.state.doc.lineAt(sel.from);
            const toLine = update.state.doc.lineAt(sel.to);
            onSelectionChange(
              sel.from,
              sel.to,
              fromLine.number,
              sel.from - fromLine.from,
              toLine.number,
              sel.to - toLine.from
            );
          }
        }
      })
    );
  }

  return new EditorView({
    state: EditorState.create({
      doc: config.content,
      extensions,
    }),
    parent: config.parent,
  });
}
