# Command Palette Design

**Date:** 2026-03-22
**Status:** Approved

## Overview

Add a command palette to Red Pen, triggered by `⌘K` (full palette) or double-`Shift` (go-to-file mode). Built with Bits UI's headless `Command` component, styled with Tailwind + existing CSS design tokens.

## Components

### `src/components/CommandPalette.svelte`

The sole new file. Wraps `Command.Root` inside a Bits UI `Dialog` for focus trapping and backdrop. Accepts props:

- `open: boolean` — controlled open state
- `onClose: () => void` — called on Esc or backdrop click
- `initialMode: "default" | "file"` — whether to open in go-to-file mode
- `onOpenSettings: () => void` — callback to show the settings dialog
- `onFileSelect: (path: string) => void` — callback to open a file

No new stores. The component calls existing exported functions directly:

- `openFile(path)` — from `$lib/stores/editor.svelte`
- `addRootFolder(path)` — from `$lib/stores/workspace.svelte`
- `expandAllFolders()`, `collapseAllFolders()`, `toggleShowChangedOnly()` — from `$lib/stores/workspace.svelte`

### `src/App.svelte` (modified)

- Add `showCommandPalette: boolean` and `commandPaletteMode: "default" | "file"` state
- Extend `handleKeydown` to handle `⌘K` and double-Shift
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

**Go to file mode:** When "Go to file…" is selected, or when opened via double-Shift, the palette switches to listing all files from the workspace `fileTree`. Bits UI's built-in search scoring filters them as the user types. Selecting a file calls `openFile(path)` and closes the palette.

## Keyboard Shortcuts

| Shortcut | Action |
|---|---|
| `⌘K` | Open palette (default mode) |
| `Shift Shift` | Open palette in go-to-file mode |
| `⌘,` | Open settings (existing, preserved) |
| `⌘↵` | Add annotation (existing, preserved) |
| `Esc` | Close palette |
| `↑↓` | Navigate items (Bits UI built-in) |
| `↵` | Execute selected item (Bits UI built-in) |

**Double-Shift detection:** Track the timestamp of the last `Shift` keyup. If a second `Shift` keyup fires within 300ms, open the palette in file mode.

## Styling

Fully Tailwind utility classes using the existing CSS custom property tokens (`bg-surface-panel`, `text-text-secondary`, `border-border-default`, etc.). No new CSS. Structure mirrors `SettingsDialog.svelte` for the overlay/backdrop pattern.

The palette is `w-[520px]`, positioned near the top-center of the viewport (`items-start pt-20`), with a max-height scrollable list.

## File Structure

```
src/
  components/
    CommandPalette.svelte   ← new
    SettingsDialog.svelte   ← unchanged
  App.svelte                ← add shortcut handlers + mount palette
```

No new dependencies. Bits UI `Command` is already available at `bits-ui@2.16.3`.
