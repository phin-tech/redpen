<script lang="ts">
let {
  checked = false,
  onToggle,
}: {
  checked: boolean;
  onToggle: () => void;
} = $props();
</script>

<button
  class="toggle-switch"
  class:toggle-switch-on={checked}
  onclick={onToggle}
  role="switch"
  aria-checked={checked}
>
  <span class="toggle-switch-thumb"></span>
</button>

<style>
.toggle-switch {
  width: 36px;
  height: 20px;
  border-radius: 10px;
  background: rgba(255, 255, 255, 0.1);
  border: none;
  padding: 2px;
  cursor: pointer;
  position: relative;
  flex-shrink: 0;
  transition: background 150ms;
}
.toggle-switch-on {
  background: var(--accent);
}
.toggle-switch-thumb {
  display: block;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: rgba(255, 255, 255, 0.4);
  transition: transform 150ms, background 150ms;
}
.toggle-switch-on .toggle-switch-thumb {
  transform: translateX(16px);
  background: white;
}
</style>
