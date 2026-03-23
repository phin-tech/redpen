export type CommandPaletteMode = "default" | "file";

export interface AppCommandContext {
  openCommandPalette: (mode: CommandPaletteMode) => void;
  openFolder: () => Promise<void>;
  openSettings: () => void;
  openAddAnnotation: () => void;
  expandAllFolders: () => Promise<void>;
  collapseAllFolders: () => void;
  toggleShowChangedOnly: () => void;
  hasRoots: () => boolean;
  canAddAnnotation: () => boolean;
  hasAnnotations: () => boolean;
  clearAnnotations: () => Promise<void>;
  isMarkdownFile: () => boolean;
  toggleMarkdownPreview: () => void;
}

export interface AppCommandDefinition {
  id: string;
  title: string;
  section: string;
  keywords: string[];
  shortcut?: string[];
  closeOnRun?: boolean;
  isEnabled?: (context: AppCommandContext) => boolean;
  run: (context: AppCommandContext) => Promise<void> | void;
}

export const COMMAND_SECTIONS = [
  "Navigation",
  "Workspace",
  "Annotations",
  "View",
] as const;

export function createCommandRegistry(): AppCommandDefinition[] {
  return [
    {
      id: "navigation.goToFile",
      title: "Go to file…",
      section: "Navigation",
      keywords: ["file", "search", "open"],
      closeOnRun: false,
      run: (context) => context.openCommandPalette("file"),
    },
    {
      id: "navigation.openFolder",
      title: "Open folder…",
      section: "Navigation",
      keywords: ["folder", "workspace", "directory"],
      run: (context) => context.openFolder(),
    },
    {
      id: "workspace.expandAll",
      title: "Expand all folders",
      section: "Workspace",
      keywords: ["expand", "folders", "tree"],
      isEnabled: (context) => context.hasRoots(),
      run: (context) => context.expandAllFolders(),
    },
    {
      id: "workspace.collapseAll",
      title: "Collapse all folders",
      section: "Workspace",
      keywords: ["collapse", "folders", "tree"],
      isEnabled: (context) => context.hasRoots(),
      run: (context) => context.collapseAllFolders(),
    },
    {
      id: "workspace.toggleChangedOnly",
      title: "Toggle changed files only",
      section: "Workspace",
      keywords: ["changed", "git", "filter"],
      isEnabled: (context) => context.hasRoots(),
      run: (context) => context.toggleShowChangedOnly(),
    },
    {
      id: "annotations.add",
      title: "Add annotation",
      section: "Annotations",
      keywords: ["annotation", "comment", "review"],
      shortcut: ["Cmd", "Return"],
      isEnabled: (context) => context.canAddAnnotation(),
      run: (context) => context.openAddAnnotation(),
    },
    {
      id: "annotations.clear",
      title: "Clear all annotations",
      section: "Annotations",
      keywords: ["clear", "remove", "delete", "reset", "annotations"],
      isEnabled: (context) => context.hasAnnotations(),
      run: (context) => context.clearAnnotations(),
    },
    {
      id: "view.toggleMarkdownPreview",
      title: "Toggle markdown preview",
      section: "View",
      keywords: ["markdown", "preview", "rendered", "md"],
      shortcut: ["Cmd", "Shift", "M"],
      isEnabled: (context) => context.isMarkdownFile(),
      run: (context) => context.toggleMarkdownPreview(),
    },
    {
      id: "view.openSettings",
      title: "Open settings",
      section: "View",
      keywords: ["settings", "preferences"],
      shortcut: ["Cmd", ","],
      run: (context) => context.openSettings(),
    },
  ];
}

export function findCommand(
  commands: AppCommandDefinition[],
  id: string
): AppCommandDefinition | undefined {
  return commands.find((command) => command.id === id);
}
