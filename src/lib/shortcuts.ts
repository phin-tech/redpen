/** A keyboard shortcut represented as an array of modifier/key tokens */
export type Shortcut = string[];

function getPlatform() {
  if (typeof navigator === "undefined") return "unknown";
  const navigatorWithUAData = navigator as Navigator & {
    userAgentData?: { platform?: string };
  };
  return navigatorWithUAData.userAgentData?.platform ?? navigator.platform ?? "unknown";
}

export function isMacPlatform(): boolean {
  return /mac|iphone|ipad|ipod/i.test(getPlatform());
}

export function formatShortcutToken(token: string): string {
  switch (token) {
    case "Mod":
      return isMacPlatform() ? "Cmd" : "Ctrl";
    case "Alt":
      return isMacPlatform() ? "Option" : "Alt";
    case "Enter":
      return isMacPlatform() ? "Return" : "Enter";
    case "Escape":
      return "Esc";
    default:
      return token.length === 1 ? token.toUpperCase() : token;
  }
}

export function formatShortcut(shortcut: Shortcut): string[] {
  return shortcut.map(formatShortcutToken);
}

function normalizeKey(key: string): string {
  switch (key) {
    case "Esc":
      return "Escape";
    case "Return":
      return "Enter";
    default:
      return key.length === 1 ? key.toUpperCase() : key;
  }
}

export function matchesShortcut(event: KeyboardEvent, shortcut: Shortcut): boolean {
  if (event.defaultPrevented || event.isComposing) return false;

  const requiresMod = shortcut.includes("Mod");
  const requiresShift = shortcut.includes("Shift");
  const requiresAlt = shortcut.includes("Alt");
  const primaryKey = shortcut.find((token) => !["Mod", "Shift", "Alt"].includes(token));

  if (!primaryKey) return false;

  const hasMod = isMacPlatform() ? event.metaKey : event.ctrlKey;
  if (hasMod !== requiresMod) return false;

  if (event.shiftKey !== requiresShift) return false;
  if (event.altKey !== requiresAlt) return false;

  if (isMacPlatform()) {
    if (!requiresMod && event.metaKey) return false;
    if (event.ctrlKey) return false;
  } else if (event.metaKey) {
    return false;
  }

  return normalizeKey(event.key) === normalizeKey(primaryKey);
}

export function isShortcutInputTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.closest(".cm-editor")) return false;
  if (target.isContentEditable) return true;
  const field = target.closest("input, textarea, select, [contenteditable='true']");
  return field !== null;
}
