import { Decoration, EditorView } from "@codemirror/view";
import { StateField, StateEffect } from "@codemirror/state";
import type { DecorationSet } from "@codemirror/view";

export const setSearchEffect = StateEffect.define<{
  matches: { from: number; to: number }[];
  currentIdx: number;
} | null>();

export const searchField = StateField.define<DecorationSet>({
  create() {
    return Decoration.none;
  },
  update(deco, tr) {
    for (const effect of tr.effects) {
      if (effect.is(setSearchEffect)) {
        if (!effect.value || effect.value.matches.length === 0) {
          return Decoration.none;
        }
        const { matches, currentIdx } = effect.value;
        return Decoration.set(
          matches.map((m, i) =>
            Decoration.mark({
              class: i === currentIdx ? "rp-search-current" : "rp-search-match",
            }).range(m.from, m.to)
          )
        );
      }
    }
    return deco;
  },
  provide: (f) => EditorView.decorations.from(f),
});

export function searchExtension() {
  return [searchField];
}
