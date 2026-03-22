<script lang="ts">
  import type { AnnotationFilter } from "$lib/types";
  import { getAnnotationsState, setFilter } from "$lib/stores/annotations.svelte";
  import { ButtonGroup, Button } from "flowbite-svelte";

  const annotationsState = getAnnotationsState();
  let allAnnotations = $derived(annotationsState.sidecar?.annotations ?? []);

  function countForFilter(f: AnnotationFilter): number {
    if (f === "all") return allAnnotations.length;
    return allAnnotations.filter((a) => a.kind === f).length;
  }

  const filters: { value: AnnotationFilter; label: string }[] = [
    { value: "all", label: "All" },
    { value: "comment", label: "Comments" },
    { value: "lineNote", label: "Notes" },
    { value: "label", label: "Labels" },
  ];
</script>

<div class="px-2.5 py-2 border-b border-graphite-700">
  <ButtonGroup class="w-full">
    {#each filters as f}
      <Button
        size="xs"
        color={annotationsState.filter === f.value ? "primary" : "alternative"}
        onclick={() => setFilter(f.value)}
        class="flex-1 !text-[11px] gap-1"
      >
        {f.label}
        <span class="opacity-60 font-mono text-[10px]">{countForFilter(f.value)}</span>
      </Button>
    {/each}
  </ButtonGroup>
</div>
