<script lang="ts">
  import { getAnnotationsState, setFilter } from "$lib/stores/annotations.svelte";
  import Button from "./ui/Button.svelte";

  type AnnotationFilter = "all" | "comment" | "lineNote" | "label" | "explanation";

  const annotationsState = getAnnotationsState();
  let allAnnotations = $derived(annotationsState.sidecar?.annotations ?? []);

  function countForFilter(f: AnnotationFilter): number {
    if (f === "all") return allAnnotations.length;
    return allAnnotations.filter((a) => a.kind === f).length;
  }

  const filters: { value: AnnotationFilter; label: string }[] = [
    { value: "all", label: "All" },
    { value: "comment", label: "Comments" },
    { value: "explanation", label: "Explanations" },
    { value: "lineNote", label: "Notes" },
    { value: "label", label: "Labels" },
  ];
</script>

<div class="px-2.5 py-2 border-b border-border-default">
  <div class="flex w-full gap-1">
    {#each filters as f}
      <Button
        variant={annotationsState.filter === f.value ? "primary" : "secondary"}
        size="sm"
        onclick={() => setFilter(f.value)}
        class="flex-1"
      >
        {f.label}
        <span class="opacity-60 font-mono text-xs">{countForFilter(f.value)}</span>
      </Button>
    {/each}
  </div>
</div>
