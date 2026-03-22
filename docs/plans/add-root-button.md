# Plan: Add Root Folder Button

## Problem
Currently the only way to add a root folder is via drag-and-drop or deep link. There's no discoverable UI button to open a folder.

## Approach

### Option A: Button in the FileTree header
Add an "+" or folder icon button in the top section of the FileTree panel, next to the root folder labels. Clicking it opens the native folder picker dialog.

### Option B: Button in the Toolbar
Add an "Open Folder" button in the top toolbar alongside settings. More visible but takes toolbar space.

### Option C: Both
A prominent "Open Folder" in the toolbar for first-time use, plus a subtle "+" in the FileTree header for power users who already have folders open.

## Implementation

1. Use `@tauri-apps/plugin-dialog` `open()` with `directory: true` to get a folder path
2. Call the existing `addRootFolder()` store function
3. Style to match the existing dark graphite theme

## Files to change
- `src/components/Toolbar.svelte` — add button (Option B/C)
- `src/components/FileTree.svelte` — add button (Option A/C)
