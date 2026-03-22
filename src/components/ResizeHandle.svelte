<script lang="ts">
  let {
    onResize,
    direction = "horizontal",
  }: {
    onResize: (delta: number) => void;
    direction?: "horizontal" | "vertical";
  } = $props();

  let dragging = $state(false);

  function onPointerDown(e: PointerEvent) {
    e.preventDefault();
    dragging = true;
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
  }

  function onPointerMove(e: PointerEvent) {
    if (!dragging) return;
    const delta = direction === "horizontal" ? e.movementX : e.movementY;
    onResize(delta);
  }

  function onPointerUp() {
    dragging = false;
  }
</script>

<div
  class="resize-handle"
  class:horizontal={direction === "horizontal"}
  class:vertical={direction === "vertical"}
  class:dragging
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  role="separator"
  aria-orientation={direction}
></div>

<style>
  .resize-handle {
    flex-shrink: 0;
    background: transparent;
    position: relative;
    z-index: 10;
  }

  .resize-handle.horizontal {
    width: 4px;
    cursor: col-resize;
    margin: 0 -2px;
  }

  .resize-handle.vertical {
    height: 4px;
    cursor: row-resize;
    margin: -2px 0;
  }

  .resize-handle:hover,
  .resize-handle.dragging {
    background: var(--accent-blue);
    opacity: 0.5;
  }

  .resize-handle.dragging {
    opacity: 0.8;
  }
</style>
