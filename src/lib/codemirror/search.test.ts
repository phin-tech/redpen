import { describe, it, expect } from "vitest";
import { EditorState } from "@codemirror/state";
import { setSearchEffect, searchField, searchExtension } from "./search";

function makeState(doc: string) {
  return EditorState.create({ doc, extensions: searchExtension() });
}

function getDecos(state: EditorState) {
  const result: { class: string; from: number; to: number }[] = [];
  state.field(searchField).between(0, state.doc.length, (from, to, deco) => {
    result.push({ class: (deco.spec as { class: string }).class, from, to });
  });
  return result;
}

describe("searchField", () => {
  it("starts with no decorations", () => {
    const state = makeState("hello world");
    expect(state.field(searchField).size).toBe(0);
  });

  it("adds one decoration per match", () => {
    const state = makeState("hello world hello");
    const matches = [
      { from: 0, to: 5 },
      { from: 12, to: 17 },
    ];
    const updated = state.update({
      effects: setSearchEffect.of({ matches, currentIdx: 0 }),
    }).state;
    expect(updated.field(searchField).size).toBe(2);
  });

  it("marks currentIdx as rp-search-current, others as rp-search-match", () => {
    const state = makeState("hello world hello");
    const matches = [
      { from: 0, to: 5 },
      { from: 12, to: 17 },
    ];

    const withFirst = state.update({
      effects: setSearchEffect.of({ matches, currentIdx: 0 }),
    }).state;
    const decosFirst = getDecos(withFirst);
    expect(decosFirst[0].class).toBe("rp-search-current");
    expect(decosFirst[1].class).toBe("rp-search-match");

    const withSecond = withFirst.update({
      effects: setSearchEffect.of({ matches, currentIdx: 1 }),
    }).state;
    const decosSecond = getDecos(withSecond);
    expect(decosSecond[0].class).toBe("rp-search-match");
    expect(decosSecond[1].class).toBe("rp-search-current");
  });

  it("clears decorations when effect value is null", () => {
    const state = makeState("hello world");
    const withMatch = state.update({
      effects: setSearchEffect.of({ matches: [{ from: 0, to: 5 }], currentIdx: 0 }),
    }).state;
    expect(withMatch.field(searchField).size).toBe(1);

    const cleared = withMatch.update({
      effects: setSearchEffect.of(null),
    }).state;
    expect(cleared.field(searchField).size).toBe(0);
  });

  it("clears decorations when matches array is empty", () => {
    const state = makeState("hello world");
    const withMatch = state.update({
      effects: setSearchEffect.of({ matches: [{ from: 0, to: 5 }], currentIdx: 0 }),
    }).state;
    const cleared = withMatch.update({
      effects: setSearchEffect.of({ matches: [], currentIdx: 0 }),
    }).state;
    expect(cleared.field(searchField).size).toBe(0);
  });

  it("preserves decorations across unrelated transactions", () => {
    const state = makeState("hello world");
    const withMatch = state.update({
      effects: setSearchEffect.of({ matches: [{ from: 0, to: 5 }], currentIdx: 0 }),
    }).state;
    // Transaction with no search effect should not change decorations
    const unchanged = withMatch.update({ sequential: true }).state;
    expect(unchanged.field(searchField).size).toBe(1);
  });
});
