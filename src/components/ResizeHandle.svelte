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
  class="shrink-0 relative z-10 bg-transparent transition-colors
    {direction === 'horizontal' ? 'w-1 cursor-col-resize -mx-0.5' : 'h-1 cursor-row-resize -my-0.5'}
    {dragging ? 'bg-accent/70' : 'hover:bg-accent/40'}"
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  role="separator"
  aria-orientation={direction}
></div>
