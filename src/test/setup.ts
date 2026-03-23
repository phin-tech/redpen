import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

if (!HTMLElement.prototype.scrollIntoView) {
  HTMLElement.prototype.scrollIntoView = () => {};
}

afterEach(() => {
  cleanup();
});
