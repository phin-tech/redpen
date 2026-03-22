# Command Palette Design

**Date:** 2026-03-22
**Status:** Approved

## Overview

Add a command palette to Red Pen, triggered by `⌘K` (full palette) or double-`Shift` (go-to-file mode). Built with Bits UI's headless `Command` component, styled with Tailwind + existing CSS design tokens.

## Components

### `src/components/CommandPalette.svelte`

The sole new file. Uses a Bits UI `Dialog` (`Dialog.Root`, `Dialog.Portal`, `Dialog.Overlay`, `Dialog.Content`) for focus trapping and backdrop, with `Command.Root` nested inside `Dialog.Content`. `Dialog.Portal` wraps `Dialog.Overlay` + `Dialog.Content` for correct z-index/stacking-context behavior. Does **not** mirror `SettingsDialog.svelte`'s hand-rolled fixed overlay — use the Bits UI Dialog primitives instead.

Accepts props:

- `open: boolean` — controlled open state
- `onClose: () => void` — called on Esc or backdrop click
- `initialMode: "default" | "file"` — whether to open in go-to-file mode
- `onOpenSettings: () => void` — callback to show the settings dialog (keeps App.svelte in control of settings state)

No new stores. All other actions are executed by calling existing exported functions directly inside the component:

- `openFile(path)` — from `$lib/stores/editor.svelte`
- `addRootFolder(path)` *(async)* — from `$lib/stores/workspace.svelte`; must be awaited in the `onSelect` handler
- `expandAllFolders()` *(async)* — from `$lib/stores/workspace.svelte`; must be awaited in the `onSelect` handler
- `collapseAllFolders()`, `toggleShowChangedOnly()` — from `$lib/stores/workspace.svelte`

Settings is the exception: it uses a callback because `showSettings` state lives in `App.svelte` and should stay there.

**Accessing workspace files:** Call `const workspace = getWorkspace()` (from `$lib/stores/workspace.svelte`) and read `workspace.fileTree`. Flatten it for go-to-file mode by iterating all map values and collecting entries where `isDir === false`, yielding a `{ name, path }[]` list.

**Bits UI `Command` subcomponents used:** `Command.Root`, `Command.Input`, `Command.List`, `Command.Group`, `Command.GroupHeading`, `Command.GroupItems`, `Command.Item`, `Command.Empty`.

### `src/App.svelte` (modified)

- Add `showCommandPalette: boolean` and `commandPaletteMode: "default" | "file"` state
- Extend `handleKeydown` (`onkeydown`) to handle `⌘K` and `⌘,` only
- Add `⌘,` handling in `handleKeydown` to set `showSettings = true` (supplements the existing native Tauri `"open-settings"` menu event — both paths set the same state)
- Add a separate `onkeyup` handler on `svelte:window` for double-Shift detection (distinct from `handleKeydown`)
- Mount `<CommandPalette>` alongside the existing `<SettingsDialog>`

## Command Groups and Items

| Group | Command | Shortcut shown |
|---|---|---|
| Navigation | Go to file… | — |
| Navigation | Open folder… | — |
| Workspace | Expand all folders | — |
| Workspace | Collapse all folders | — |
| Workspace | Toggle changed files only | — |
| Annotations | Add annotation | `⌘↵` |
| View | Open settings | `⌘,` |

**Go to file mode:** When "Go to file…" is selected, or when opened via double-Shift, the palette switches to listing all files from the workspace. Flatten `getWorkspace().fileTree` (a `SvelteMap<string, FileEntry[]>`) by iterating all map values and collecting entries where `isDir === false`. Bits UI's built-in search scoring filters them as the user types. Selecting a file calls `openFile(path)` and closes the palette.

**Full file tree loading:** When go-to-file mode opens, call `await expandAllFolders()` before building the file list to ensure all subdirectories are loaded. This means `workspace.fileTree` will contain all files across the entire workspace, not just expanded directories.

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `⌘K` | Open palette (default mode) |
| `Shift Shift` | Open palette in go-to-file mode |
| `⌘,` | Open settings (new keyboard shortcut, added by this work; Tauri menu event preserved) |
| `⌘↵` | Add annotation (existing, preserved) |
| `Esc` | Close palette |
| `↑↓` | Navigate items (Bits UI built-in) |
| `↵` | Execute selected item (Bits UI built-in) |

**Double-Shift detection:** Add a separate `onkeyup` handler on `svelte:window` (distinct from the existing `onkeydown` handler). Track the timestamp of the last `Shift` keyup. If a second `Shift` keyup fires within 300ms, open the palette in file mode.

## Styling

Fully Tailwind utility classes using the existing CSS custom property tokens (`bg-surface-panel`, `text-text-secondary`, `border-border-default`, etc.). No new CSS.

The palette is `w-[520px]`, positioned near the top-center of the viewport (`items-start pt-20`), with a max-height scrollable list.

## File Structure

```
src/
  components/
    CommandPalette.svelte   ← new
    SettingsDialog.svelte   ← unchanged
  App.svelte                ← add shortcut handlers + mount palette
```

No new dependencies. Bits UI `Command` and `Dialog` are already available at `bits-ui@2.16.3`.
