<script lang="ts">
  import { getSettings, updateSettings, getGitRemoteUrl } from "$lib/tauri";
  import { parseGitHubRepo } from "$lib/utils/parseGitRemote";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import TagInput from "./ui/TagInput.svelte";
  import ToggleSwitch from "./ui/ToggleSwitch.svelte";

  let { onClose }: { onClose: () => void } = $props();

  let activeCategory = $state<"general" | "git" | "notifications">("general");

  let author = $state("");
  let labelsList = $state<string[]>([]);
  let ignoredFoldersList = $state<string[]>([]);
  let defaultCheckoutRoot = $state("");
  let trackedReposList = $state<{ repo: string; localPath: string }[]>([]);
  let newRepoName = $state("");
  let newRepoPath = $state("");
  let notifyAnnotationReply = $state(true);
  let notifyReviewComplete = $state(true);
  let notifyNewAnnotation = $state(false);
  let notifyDeepLink = $state(true);

  onMount(async () => {
    const settings = await getSettings();
    author = settings.author;
    labelsList = settings.defaultLabels ?? [];
    ignoredFoldersList = settings.ignoredFolderNames ?? [];
    defaultCheckoutRoot = settings.defaultCheckoutRoot ?? "";
    trackedReposList = settings.trackedGithubRepos ?? [];
    if (settings.notifications) {
      notifyAnnotationReply = settings.notifications.annotationReply;
      notifyReviewComplete = settings.notifications.reviewComplete;
      notifyNewAnnotation = settings.notifications.newAnnotation;
      notifyDeepLink = settings.notifications.deepLink;
    }
  });

  async function save() {
    await updateSettings({
      author,
      defaultLabels: labelsList,
      ignoredFolderNames: ignoredFoldersList,
      defaultCheckoutRoot: defaultCheckoutRoot.trim(),
      trackedGithubRepos: trackedReposList,
      notifications: {
        annotationReply: notifyAnnotationReply,
        reviewComplete: notifyReviewComplete,
        newAnnotation: notifyNewAnnotation,
        deepLink: notifyDeepLink,
      },
    });
    onClose();
  }

  async function browseCheckoutRoot() {
    const selected = await openDialog({ directory: true });
    if (selected && typeof selected === "string") {
      defaultCheckoutRoot = selected;
    }
  }

  async function browseRepoPath() {
    const selected = await openDialog({ directory: true });
    if (selected && typeof selected === "string") {
      newRepoPath = selected;
    }
  }

  function addRepo() {
    if (!newRepoName.trim() || !newRepoPath.trim()) return;
    trackedReposList = [...trackedReposList, { repo: newRepoName.trim(), localPath: newRepoPath.trim() }];
    newRepoName = "";
    newRepoPath = "";
  }

  function removeRepo(index: number) {
    trackedReposList = trackedReposList.filter((_, i) => i !== index);
  }

  async function addRepoFromDisk() {
    const selected = await openDialog({ directory: true });
    if (!selected || typeof selected !== "string") return;

    const remoteUrl = await getGitRemoteUrl(selected);
    if (remoteUrl) {
      const parsed = parseGitHubRepo(remoteUrl);
      if (parsed) {
        trackedReposList = [...trackedReposList, { repo: parsed, localPath: selected }];
        return;
      }
    }
    // Fallback: pre-fill path, user types repo name
    newRepoPath = selected;
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) save();
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="settings-backdrop" onclick={onClose}>
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div
    class="settings-dialog"
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleKeydown}
    role="dialog"
    tabindex="-1"
  >
    <!-- Sidebar -->
    <div class="settings-sidebar">
      <div class="settings-sidebar-title">Settings</div>
      <button
        class="settings-sidebar-btn"
        class:settings-sidebar-btn-active={activeCategory === "general"}
        onclick={() => activeCategory = "general"}
      >General</button>
      <button
        class="settings-sidebar-btn"
        class:settings-sidebar-btn-active={activeCategory === "git"}
        onclick={() => activeCategory = "git"}
      >Git & GitHub</button>
      <button
        class="settings-sidebar-btn"
        class:settings-sidebar-btn-active={activeCategory === "notifications"}
        onclick={() => activeCategory = "notifications"}
      >Notifications</button>
    </div>

    <!-- Content -->
    <div class="settings-main">
      <div class="settings-content">
        {#if activeCategory === "general"}
          <div class="settings-section-header">Identity</div>
          <div class="settings-field">
            <label>Author name</label>
            <input bind:value={author} />
          </div>

          <div class="settings-section-header">Annotation Defaults</div>
          <div class="settings-field">
            <label>Default labels</label>
            <TagInput
              tags={labelsList}
              onAdd={(tag) => { labelsList = [...labelsList, tag]; }}
              onRemove={(i) => { labelsList = labelsList.filter((_, idx) => idx !== i); }}
              placeholder="Add label..."
            />
          </div>
          <div class="settings-field">
            <label>Ignored folders</label>
            <TagInput
              tags={ignoredFoldersList}
              onAdd={(tag) => { ignoredFoldersList = [...ignoredFoldersList, tag]; }}
              onRemove={(i) => { ignoredFoldersList = ignoredFoldersList.filter((_, idx) => idx !== i); }}
              placeholder="Add folder..."
              mono
            />
          </div>

        {:else if activeCategory === "git"}
          <div class="settings-section-header">Checkout</div>
          <div class="settings-field">
            <label>Default checkout location</label>
            <div class="settings-path-picker">
              <div class="settings-path-display">{defaultCheckoutRoot || "Not set"}</div>
              <button class="settings-browse-btn" onclick={browseCheckoutRoot}>Browse...</button>
            </div>
            <p class="settings-hint">Used when opening a PR for a repo without a tracked checkout.</p>
          </div>

          <div class="settings-section-header">Tracked Repositories</div>
          {#each trackedReposList as repo, i}
            <div class="settings-repo-card">
              <div class="settings-repo-info">
                <div class="settings-repo-name">{repo.repo}</div>
                <div class="settings-repo-path">{repo.localPath}</div>
              </div>
              <button class="settings-repo-remove" onclick={() => removeRepo(i)}>&times;</button>
            </div>
          {/each}

          <div class="settings-add-repo">
            <input bind:value={newRepoName} placeholder="owner/repo" />
            <div class="settings-path-picker-inline">
              <input bind:value={newRepoPath} placeholder="/path/to/checkout" />
              <button onclick={browseRepoPath}>&#x1F4C1;</button>
            </div>
            <button onclick={addRepo}>Add</button>
          </div>

          <div class="settings-divider-text">or</div>

          <button class="settings-add-from-disk" onclick={addRepoFromDisk}>
            &#x1F4C1; Add from disk...
          </button>
          <p class="settings-hint" style="text-align:center">Opens a folder picker. Detects the GitHub remote automatically.</p>

        {:else if activeCategory === "notifications"}
          <div class="settings-section-header">Desktop Notifications</div>

          <div class="settings-toggle-row">
            <div class="settings-toggle-label">
              <div>Agent replied to annotation</div>
              <div class="settings-toggle-desc">When an agent responds to one of your comments</div>
            </div>
            <ToggleSwitch checked={notifyAnnotationReply} onToggle={() => notifyAnnotationReply = !notifyAnnotationReply} />
          </div>

          <div class="settings-toggle-row">
            <div class="settings-toggle-label">
              <div>Review complete</div>
              <div class="settings-toggle-desc">When a review session finishes</div>
            </div>
            <ToggleSwitch checked={notifyReviewComplete} onToggle={() => notifyReviewComplete = !notifyReviewComplete} />
          </div>

          <div class="settings-toggle-row">
            <div class="settings-toggle-label">
              <div>New annotation on file</div>
              <div class="settings-toggle-desc">When someone adds an annotation to a file you're watching</div>
            </div>
            <ToggleSwitch checked={notifyNewAnnotation} onToggle={() => notifyNewAnnotation = !notifyNewAnnotation} />
          </div>

          <div class="settings-toggle-row">
            <div class="settings-toggle-label">
              <div>Deep link received</div>
              <div class="settings-toggle-desc">When the app is opened via a deep link</div>
            </div>
            <ToggleSwitch checked={notifyDeepLink} onToggle={() => notifyDeepLink = !notifyDeepLink} />
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="settings-footer">
        <button class="settings-cancel-btn" onclick={onClose}>Cancel</button>
        <button class="settings-save-btn" onclick={save}>Save Changes</button>
      </div>
    </div>
  </div>
</div>

<style>
.settings-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.settings-dialog {
  width: 680px;
  max-height: 80vh;
  display: flex;
  flex-direction: row;
  border: 1px solid color-mix(in srgb, var(--border-default) 60%, transparent);
  border-radius: 12px;
  background: var(--surface-panel);
  box-shadow: var(--shadow-popover), 0 0 0 1px var(--border-subtle);
  overflow: hidden;
}

/* Sidebar */
.settings-sidebar {
  width: 140px;
  flex-shrink: 0;
  background: var(--surface-panel);
  border-right: 1px solid var(--border-default);
  padding: 16px 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.settings-sidebar-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  padding: 0 14px 12px;
}

.settings-sidebar-btn {
  display: block;
  width: 100%;
  text-align: left;
  padding: 8px 14px;
  font-size: 12px;
  color: var(--text-secondary);
  background: transparent;
  border: none;
  border-left: 2px solid transparent;
  cursor: pointer;
  transition: background 100ms, color 100ms;
  font-family: inherit;
}

.settings-sidebar-btn:hover {
  background: rgba(255, 255, 255, 0.04);
  color: var(--text-primary);
}

.settings-sidebar-btn-active {
  border-left-color: var(--accent);
  background: rgba(255, 255, 255, 0.06);
  color: var(--text-primary);
  font-weight: 500;
}

/* Main area */
.settings-main {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.settings-content {
  flex: 1;
  padding: 20px;
  overflow-y: auto;
}

/* Section headers */
.settings-section-header {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  border-bottom: 1px solid var(--border-default);
  padding-bottom: 6px;
  margin-bottom: 14px;
  margin-top: 8px;
}

.settings-section-header:first-child {
  margin-top: 0;
}

/* Fields */
.settings-field {
  margin-bottom: 16px;
}

.settings-field label {
  display: block;
  font-size: 12px;
  font-weight: 500;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.settings-field input {
  width: 100%;
  background: var(--surface-base);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 150ms;
  box-sizing: border-box;
}

.settings-field input:focus {
  border-color: var(--accent);
}

/* Path picker */
.settings-path-picker {
  display: flex;
  gap: 6px;
  align-items: stretch;
}

.settings-path-display {
  flex: 1;
  background: var(--surface-base);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 7px 10px;
  color: var(--text-muted);
  font-size: 12px;
  font-family: var(--font-mono, monospace);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings-browse-btn {
  background: transparent;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 7px 12px;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  font-family: inherit;
  transition: background 100ms;
}

.settings-browse-btn:hover {
  background: rgba(255, 255, 255, 0.04);
}

/* Hints */
.settings-hint {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 6px;
  margin-bottom: 0;
}

/* Repo cards */
.settings-repo-card {
  background: var(--surface-panel);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  display: flex;
  align-items: center;
  padding: 8px 10px;
  gap: 10px;
  margin-bottom: 6px;
}

.settings-repo-info {
  flex: 1;
  min-width: 0;
}

.settings-repo-name {
  font-size: 12px;
  font-weight: 500;
  color: var(--text-primary);
}

.settings-repo-path {
  font-size: 10px;
  font-family: var(--font-mono, monospace);
  color: var(--text-muted);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.settings-repo-remove {
  width: 24px;
  height: 24px;
  background: transparent;
  border: none;
  color: var(--text-ghost);
  font-size: 16px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  flex-shrink: 0;
  transition: color 100ms;
  font-family: inherit;
}

.settings-repo-remove:hover {
  color: var(--text-muted);
}

/* Add repo form */
.settings-add-repo {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-top: 10px;
}

.settings-add-repo input {
  width: 100%;
  background: var(--surface-base);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 7px 10px;
  color: var(--text-primary);
  font-size: 12px;
  font-family: inherit;
  outline: none;
  transition: border-color 150ms;
  box-sizing: border-box;
}

.settings-add-repo input:focus {
  border-color: var(--accent);
}

.settings-path-picker-inline {
  display: flex;
  gap: 6px;
}

.settings-path-picker-inline input {
  flex: 1;
}

.settings-path-picker-inline button {
  background: transparent;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 7px 10px;
  cursor: pointer;
  font-size: 14px;
  line-height: 1;
  transition: background 100ms;
}

.settings-path-picker-inline button:hover {
  background: rgba(255, 255, 255, 0.04);
}

.settings-add-repo > button:last-child {
  align-self: flex-end;
  background: transparent;
  border: 1px solid var(--border-default);
  border-radius: 6px;
  padding: 6px 16px;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  font-family: inherit;
  transition: background 100ms;
}

.settings-add-repo > button:last-child:hover {
  background: rgba(255, 255, 255, 0.04);
}

.settings-divider-text {
  text-align: center;
  font-size: 11px;
  color: var(--text-ghost);
  margin: 12px 0;
}

.settings-add-from-disk {
  width: 100%;
  background: var(--surface-panel);
  border: 1px dashed var(--border-default);
  border-radius: 6px;
  padding: 10px;
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
  text-align: center;
  font-family: inherit;
  transition: background 100ms;
}

.settings-add-from-disk:hover {
  background: rgba(255, 255, 255, 0.04);
}

/* Toggle rows */
.settings-toggle-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border-default) 40%, transparent);
}

.settings-toggle-row:last-of-type {
  border-bottom: none;
}

.settings-toggle-label {
  text-align: left;
}

.settings-toggle-label > div:first-child {
  font-size: 13px;
  color: var(--text-primary);
}

.settings-toggle-desc {
  font-size: 11px;
  color: var(--text-muted);
  margin-top: 2px;
}

/* Footer */
.settings-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 20px;
  border-top: 1px solid var(--border-default);
}

.settings-cancel-btn {
  background: transparent;
  border: none;
  color: var(--text-secondary);
  font-size: 12px;
  padding: 6px 14px;
  cursor: pointer;
  border-radius: 6px;
  font-family: inherit;
  transition: color 100ms;
}

.settings-cancel-btn:hover {
  color: var(--text-primary);
}

.settings-save-btn {
  background: var(--accent);
  border: none;
  color: #000;
  font-size: 12px;
  font-weight: 500;
  padding: 6px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-family: inherit;
  transition: opacity 150ms;
}

.settings-save-btn:hover {
  opacity: 0.9;
}
</style>
