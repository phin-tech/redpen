<script lang="ts">
  import type { Annotation } from "$lib/types";
  import { splitMarkdownBlocks } from "$lib/markdown/blocks";
  import { renderMarkdown } from "$lib/markdown/render";
  import "$lib/markdown/markdown.css";
  import {
    loadAnnotations,
    sortedAnnotations,
    addAnnotation,
    selectAnnotation,
    removeAnnotation,
    getAnnotationsState,
  } from "$lib/stores/annotations.svelte";
  import { updateChoices } from "$lib/stores/annotations.svelte";
  import AnnotationPopover from "./AnnotationPopover.svelte";
  import AnnotationBubble from "./AnnotationBubble.svelte";

  let {
    body,
    worktreePath,
  }: {
    body: string;
    worktreePath: string;
  } = $props();

  const virtualFilePath = $derived(`${worktreePath}/__redpen__/pr-body.md`);

  const blocks = $derived(splitMarkdownBlocks(body || ""));

  let expandedLine = $state<number | null>(null);
  let popoverLine = $state<number | null>(null);
  let popoverPosition = $state({ x: 0, y: 0 });

  // Load annotations for the virtual file on mount / when path changes
  $effect(() => {
    if (virtualFilePath) {
      loadAnnotations(virtualFilePath);
    }
  });

  const annotations = $derived(sortedAnnotations());

  const annotationCount = $derived(annotations.filter((a) => !a.replyTo).length);

  // Group annotations by line number (root annotations with their replies)
  const annotationsByLine = $derived.by(() => {
    const map = new Map<number, Annotation[]>();
    const roots = annotations.filter((a) => !a.replyTo);
    const replies = annotations.filter((a) => a.replyTo);

    for (const root of roots) {
      const line = root.anchor.range.startLine;
      if (!map.has(line)) map.set(line, []);
      map.get(line)!.push(root);
      // Add replies for this root
      for (const reply of replies) {
        if (reply.replyTo === root.id) {
          map.get(line)!.push(reply);
        }
      }
    }
    return map;
  });

  // Lines that have annotations, sorted, for numbering dots
  const annotatedLines = $derived.by(() => {
    const lines = [...annotationsByLine.keys()].sort((a, b) => a - b);
    return lines;
  });

  function getDotNumber(lineNumber: number): number {
    return annotatedLines.indexOf(lineNumber) + 1;
  }

  function handleAddClick(lineNumber: number, event: MouseEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    popoverPosition = { x: rect.right + 8, y: rect.top };
    popoverLine = lineNumber;
  }

  async function handlePopoverSubmit(bodyText: string, labels: string[]) {
    if (popoverLine === null) return;
    await addAnnotation(
      virtualFilePath,
      bodyText,
      labels,
      popoverLine,
      0,
      popoverLine,
      0,
    );
    popoverLine = null;
  }

  function handlePopoverCancel() {
    popoverLine = null;
  }

  function handleToggle(lineNumber: number) {
    expandedLine = expandedLine === lineNumber ? null : lineNumber;
  }

  function handleSelect(id: string) {
    selectAnnotation(id);
  }

  async function handleDelete(id: string) {
    await removeAnnotation(virtualFilePath, id);
  }

  async function handleChoiceToggle(annotationId: string, choiceIndex: number) {
    const ann = annotations.find((a) => a.id === annotationId);
    if (!ann || !ann.choices) return;
    const newChoices = ann.choices.map((c, i) => ({
      ...c,
      selected: i === choiceIndex ? !c.selected : c.selected,
    }));
    await updateChoices(virtualFilePath, annotationId, newChoices);
  }

  function renderBlockHtml(content: string): string {
    return renderMarkdown(content);
  }
</script>

<div class="pr-body-annotatable markdown-body">
  {#if !body || body.trim() === ""}
    <p class="pr-body-empty"><em>No description provided.</em></p>
  {:else}
    {#each blocks as block}
      {@const lineAnnotations = annotationsByLine.get(block.lineNumber) ?? []}
      {@const hasAnnotations = lineAnnotations.length > 0}
      {@const dotNumber = hasAnnotations ? getDotNumber(block.lineNumber) : 0}
      {@const isExpanded = expandedLine === block.lineNumber}

      <div
        class="pr-body-block"
        class:pr-body-block-hover={!hasAnnotations}
        class:pr-body-block-annotated={hasAnnotations}
      >
        <div class="pr-body-gutter">
          {#if hasAnnotations}
            <button
              class="pr-body-dot"
              class:pr-body-dot-focused={isExpanded}
              onclick={() => handleToggle(block.lineNumber)}
              title="Toggle annotation"
            >
              {dotNumber}
            </button>
          {:else}
            <button
              class="pr-body-add-btn"
              onclick={(e) => handleAddClick(block.lineNumber, e)}
              title="Add annotation"
            >+</button>
          {/if}
        </div>
        <div class="pr-body-content">
          {@html renderBlockHtml(block.content)}
        </div>
      </div>

      {#if hasAnnotations && isExpanded}
        <div class="pr-body-bubble-wrapper">
          <AnnotationBubble
            annotations={lineAnnotations}
            expanded={true}
            focusPosition={{ current: dotNumber, total: annotatedLines.length }}
            onToggle={() => handleToggle(block.lineNumber)}
            onSelect={handleSelect}
            onDelete={handleDelete}
            onChoiceToggle={handleChoiceToggle}
          />
        </div>
      {:else if hasAnnotations && !isExpanded}
        <div class="pr-body-bubble-wrapper">
          <AnnotationBubble
            annotations={lineAnnotations}
            expanded={false}
            onToggle={() => handleToggle(block.lineNumber)}
            onSelect={handleSelect}
            onDelete={handleDelete}
            onChoiceToggle={handleChoiceToggle}
          />
        </div>
      {/if}
    {/each}
  {/if}

  {#if popoverLine !== null}
    <AnnotationPopover
      position={popoverPosition}
      onSubmit={handlePopoverSubmit}
      onCancel={handlePopoverCancel}
    />
  {/if}
</div>

<style>
  .pr-body-annotatable {
    /* padding provided by parent container */
  }

  .pr-body-empty {
    color: var(--text-muted);
    font-style: italic;
  }

  .pr-body-block {
    position: relative;
    padding-left: 28px;
    border-radius: 4px;
    transition: background 0.1s ease;
  }

  .pr-body-block-hover:hover {
    background: rgba(217, 177, 95, 0.03);
  }

  .pr-body-block-annotated {
    background: rgba(217, 177, 95, 0.03);
  }

  .pr-body-gutter {
    position: absolute;
    left: 0;
    top: 0;
    width: 24px;
    height: 100%;
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 4px;
  }

  .pr-body-dot {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: rgba(217, 177, 95, 0.2);
    color: var(--text-primary);
    font-size: 10px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    cursor: pointer;
    transition: background 0.15s ease;
    line-height: 1;
  }

  .pr-body-dot:hover,
  .pr-body-dot-focused {
    background: rgb(217, 177, 95);
    color: #1a1a1a;
  }

  .pr-body-add-btn {
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: rgb(217, 177, 95);
    color: #1a1a1a;
    font-size: 12px;
    font-weight: 600;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.15s ease;
    line-height: 1;
    padding: 0;
  }

  .pr-body-block:hover .pr-body-add-btn {
    opacity: 1;
  }

  .pr-body-bubble-wrapper {
    padding-left: 28px;
    margin-bottom: 8px;
  }

  .pr-body-content :global(> *:first-child) {
    margin-top: 0;
  }

  .pr-body-content :global(> *:last-child) {
    margin-bottom: 0;
  }
</style>
