import { describe, expect, it, beforeEach } from "vitest";
import { getReviewSession, addReviewFile, clearReviewSession } from "./review.svelte";

describe("review session store", () => {
  beforeEach(() => {
    clearReviewSession();
  });

  it("starts inactive with no files", () => {
    const session = getReviewSession();
    expect(session.active).toBe(false);
    expect(session.files).toEqual([]);
  });

  it("adds files and becomes active", () => {
    addReviewFile("/path/to/file.ts");
    const session = getReviewSession();
    expect(session.active).toBe(true);
    expect(session.files).toEqual(["/path/to/file.ts"]);
  });

  it("does not add duplicate files", () => {
    addReviewFile("/path/to/file.ts");
    addReviewFile("/path/to/file.ts");
    const session = getReviewSession();
    expect(session.files).toEqual(["/path/to/file.ts"]);
  });

  it("adds multiple unique files", () => {
    addReviewFile("/path/to/a.ts");
    addReviewFile("/path/to/b.ts");
    const session = getReviewSession();
    expect(session.files).toHaveLength(2);
  });

  it("clears session", () => {
    addReviewFile("/path/to/file.ts");
    clearReviewSession();
    const session = getReviewSession();
    expect(session.active).toBe(false);
    expect(session.files).toEqual([]);
  });
});
