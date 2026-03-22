import { SvelteMap, SvelteSet } from "svelte/reactivity";
import { readDirectory, getGitStatus } from "$lib/tauri";
import type { FileEntry, GitFileStatus } from "$lib/types";

interface WorkspaceState {
  rootFolders: string[];
  fileTree: SvelteMap<string, FileEntry[]>;
  gitStatuses: SvelteMap<string, GitFileStatus[]>;
  expandedFolders: SvelteSet<string>;
  showChangedOnly: boolean;
}

let state = $state<WorkspaceState>({
  rootFolders: [],
  fileTree: new SvelteMap(),
  gitStatuses: new SvelteMap(),
  expandedFolders: new SvelteSet(),
  showChangedOnly: false,
});

export function getWorkspace() {
  return state;
}

export async function addRootFolder(path: string) {
  if (state.rootFolders.includes(path)) return;
  state.rootFolders = [...state.rootFolders, path];
  await loadDirectory(path);
  await loadGitStatus(path);
}

export async function removeRootFolder(path: string) {
  state.rootFolders = state.rootFolders.filter((f) => f !== path);
  state.fileTree.delete(path);
  state.gitStatuses.delete(path);
}

export async function loadDirectory(path: string) {
  const entries = await readDirectory(path);
  state.fileTree.set(path, entries);
}

export async function loadGitStatus(directory: string) {
  const statuses = await getGitStatus(directory);
  state.gitStatuses.set(directory, statuses);
}

export function toggleFolder(path: string) {
  if (state.expandedFolders.has(path)) {
    state.expandedFolders.delete(path);
  } else {
    state.expandedFolders.add(path);
    if (!state.fileTree.has(path)) {
      loadDirectory(path);
    }
  }
}

export function getGitStatusForFile(filePath: string): GitFileStatus | undefined {
  for (const [, statuses] of state.gitStatuses) {
    const match = statuses.find((s) => filePath.endsWith(s.path));
    if (match) return match;
  }
  return undefined;
}

export function toggleShowChangedOnly() {
  state.showChangedOnly = !state.showChangedOnly;
  if (state.showChangedOnly) {
    // Auto-expand directories containing changed files
    const paths = getChangedFilePaths();
    for (const p of paths) {
      // Expand directories that are ancestors of changed files
      if (!state.fileTree.has(p)) continue;
      state.expandedFolders.add(p);
    }
  }
}

export function getChangedFilePaths(): Set<string> {
  const paths = new Set<string>();

  for (const [rootDir, statuses] of state.gitStatuses) {
    for (const status of statuses) {
      // Build absolute path from root directory + relative git path
      const absPath = `${rootDir}/${status.path}`;
      paths.add(absPath);

      // Add all ancestor directories up to (and including) the root
      let dir = absPath.substring(0, absPath.lastIndexOf("/"));
      while (dir.length >= rootDir.length) {
        paths.add(dir);
        const parent = dir.substring(0, dir.lastIndexOf("/"));
        if (parent === dir) break;
        dir = parent;
      }
    }
  }

  return paths;
}
