<script lang="ts">
  let {
    logText,
    onFileLineClick,
  }: {
    logText: string | null;
    onFileLineClick: (file: string, line: number) => void;
  } = $props();

  interface FileRef {
    file: string;
    basename: string;
    lines: number[];
  }

  // Extract file:line references from log text
  const fileRefs = $derived((): FileRef[] => {
    if (!logText) return [];
    const refs = new Map<string, Set<number>>();

    for (const line of logText.split("\n")) {
      // Rust: --> path/to/file.rs:123:45 or --> path/to/file.rs:123
      let match = line.match(/-->\s+([^\s:]+\.[a-z]+):(\d+)/);
      if (match && match[1].includes("/")) {
        addRef(refs, match[1], parseInt(match[2]));
        continue;
      }

      // TypeScript: path/to/file.ts(123,45): error
      match = line.match(/([^\s(]+\.[a-z]+)\((\d+),\d+\):\s*error/);
      if (match && match[1].includes("/")) {
        addRef(refs, match[1], parseInt(match[2]));
        continue;
      }

      // Generic: path/to/file.ext:123 (must contain / and be preceded by whitespace or start of line)
      match = line.match(/(?:^|\s)([a-zA-Z0-9_./-]+\/[a-zA-Z0-9_./-]+\.[a-z]+):(\d+)/);
      if (match) {
        addRef(refs, match[1], parseInt(match[2]));
      }
    }

    return Array.from(refs.entries())
      .map(([file, lines]) => ({
        file,
        basename: file.split("/").pop() || file,
        lines: Array.from(lines).sort((a, b) => a - b),
      }))
      .sort((a, b) => a.file.localeCompare(b.file));
  });

  function addRef(refs: Map<string, Set<number>>, file: string, line: number) {
    if (!refs.has(file)) refs.set(file, new Set());
    refs.get(file)!.add(line);
  }

  const hasRefs = $derived(() => fileRefs().length > 0);
</script>

{#if hasRefs()}
  <div class="error-context">
    <div class="context-header">
      <span class="context-title">Failing Files</span>
    </div>
    <div class="context-scroll">
      {#each fileRefs() as ref (ref.file)}
        <div class="file-group">
          <div class="file-name">{ref.basename}</div>
          <div class="line-links">
            {#each ref.lines as line}
              <button
                class="line-link"
                onclick={() => onFileLineClick(ref.file, line)}
              >:{line}</button>
            {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .error-context {
    width: 200px;
    min-width: 200px;
    display: flex;
    flex-direction: column;
    background: var(--surface-panel);
    border-left: 1px solid var(--border-default);
    height: 100%;
  }
  .context-header {
    padding: 10px 12px;
    border-bottom: 1px solid var(--border-default);
  }
  .context-title {
    font-size: 10px;
    color: var(--text-muted);
    text-transform: uppercase;
    font-weight: 700;
    letter-spacing: 0.05em;
  }
  .context-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px;
  }
  .file-group {
    padding: 6px 8px;
    background: color-mix(in srgb, var(--color-danger) 8%, transparent);
    border-radius: 4px;
    margin-bottom: 4px;
  }
  .file-name {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-primary);
    margin-bottom: 4px;
  }
  .line-links {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .line-link {
    font-size: 10px;
    font-family: var(--font-mono);
    color: var(--view-active, #6366f1);
    background: none;
    border: none;
    padding: 0 2px;
    cursor: pointer;
    text-decoration: none;
  }
  .line-link:hover {
    text-decoration: underline;
  }
</style>
