<script lang="ts">
let {
  tags = [],
  onAdd,
  onRemove,
  placeholder = "Add...",
  mono = false,
}: {
  tags: string[];
  onAdd: (tag: string) => void;
  onRemove: (index: number) => void;
  placeholder?: string;
  mono?: boolean;
} = $props();

let inputValue = $state("");
let focused = $state(false);

function handleKeydown(e: KeyboardEvent) {
  if (e.key === "Enter" || e.key === ",") {
    e.preventDefault();
    const trimmed = inputValue.trim();
    if (trimmed) {
      onAdd(trimmed);
      inputValue = "";
    }
  } else if (e.key === "Backspace" && inputValue === "" && tags.length > 0) {
    onRemove(tags.length - 1);
  }
}
</script>

<div class="tag-input" class:tag-input-focused={focused} class:tag-input-mono={mono}>
  {#each tags as tag, i}
    <span class="tag-pill">
      {tag}
      <button class="tag-remove" onclick={() => onRemove(i)}>&times;</button>
    </span>
  {/each}
  <input
    class="tag-text-input"
    bind:value={inputValue}
    {placeholder}
    onfocus={() => focused = true}
    onblur={() => focused = false}
    onkeydown={handleKeydown}
  />
</div>

<style>
.tag-input {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  padding: 5px 8px;
  background: var(--surface-base);
  border: 1px solid var(--border-default);
  border-radius: 6px;
  min-height: 32px;
  align-items: center;
  cursor: text;
}
.tag-input-focused {
  border-color: var(--accent);
}
.tag-pill {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: var(--surface-raised);
  color: var(--text-secondary);
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
}
.tag-input-mono .tag-pill,
.tag-input-mono .tag-text-input {
  font-family: var(--font-mono);
}
.tag-remove {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  padding: 0;
  font-size: 12px;
  line-height: 1;
  font-family: inherit;
}
.tag-remove:hover {
  color: var(--text-primary);
}
.tag-text-input {
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 11px;
  outline: none;
  flex: 1;
  min-width: 60px;
  padding: 2px 0;
  font-family: inherit;
}
.tag-text-input::placeholder {
  color: var(--text-muted);
}
</style>
