import { SvelteMap, SvelteSet } from "svelte/reactivity";
import {
  readDirectory,
  getGitStatus,
  getWorkspaceIndexStatus,
  registerWorkspaceRoot,
  unregisterWorkspaceRoot,
} from "$lib/tauri";
import type { FileEntry, GitFileStatus, WorkspaceIndexStatus } from "$lib/types";

interface WorkspaceState {
  rootFolders: string[];
  fileTree: SvelteMap<string, FileEntry[]>;
  gitStatuses: SvelteMap<string, GitFileStatus[]>;
  indexStatuses: SvelteMap<string, WorkspaceIndexStatus>;
  expandedFolders: SvelteSet<string>;
  showChangedOnly: boolean;
}

let state = $state<WorkspaceState>({
  rootFolders: [],
  fileTree: new SvelteMap(),
  gitStatuses: new SvelteMap(),
  indexStatuses: new SvelteMap(),
  expandedFolders: new SvelteSet(),
  showChangedOnly: false,
});

export function getWorkspace() {
  return state;
}

export function resetWorkspaceForTests() {
  state.rootFolders = [];
  state.fileTree.clear();
  state.gitStatuses.clear();
  state.indexStatuses.clear();
  state.expandedFolders.clear();
  state.showChangedOnly = false;
}

export async function addRootFolder(path: string) {
  if (state.rootFolders.includes(path)) return;
  await registerWorkspaceRoot(path);
  state.rootFolders = [...state.rootFolders, path];
  await Promise.all([loadDirectory(path), loadGitStatus(path), refreshWorkspaceIndexStatus([path])]);
}

export async function removeRootFolder(path: string) {
  await unregisterWorkspaceRoot(path);
  state.rootFolders = state.rootFolders.filter((f) => f !== path);
  state.fileTree.delete(path);
  state.gitStatuses.delete(path);
  state.indexStatuses.delete(path);
}

export async function loadDirectory(path: string) {
  const entries = await readDirectory(path);
  state.fileTree.set(path, entries);
}

export async function loadGitStatus(directory: string) {
  const statuses = await getGitStatus(directory);
  state.gitStatuses.set(directory, statuses);
}

export async function refreshWorkspaceIndexStatus(roots?: string[]) {
  const statuses = await getWorkspaceIndexStatus(roots);
  const requestedRoots = roots ? new Set(roots) : null;

  if (requestedRoots) {
    for (const root of requestedRoots) {
      state.indexStatuses.delete(root);
    }
  } else {
    state.indexStatuses.clear();
  }

  for (const status of statuses) {
    state.indexStatuses.set(status.root, status);
  }
}

export function getWorkspaceIndexStatusForRoot(root: string): WorkspaceIndexStatus | undefined {
  return state.indexStatuses.get(root);
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

export async function expandAllFolders() {
  async function expandRecursive(path: string) {
    state.expandedFolders.add(path);
    if (!state.fileTree.has(path)) {
      await loadDirectory(path);
    }
    const entries = state.fileTree.get(path) ?? [];
    for (const entry of entries) {
      if (entry.isDir) {
        await expandRecursive(entry.path);
      }
    }
  }

  for (const root of state.rootFolders) {
    await expandRecursive(root);
  }
}

export function collapseAllFolders() {
  state.expandedFolders.clear();
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
