import { getAnnotations } from "$lib/tauri";
import { splitMarkdownBlocks, blockPreview } from "$lib/markdown/blocks";
import type { Annotation } from "$lib/types";

/**
 * Collect PR body annotations and format as markdown for the review summary.
 */
export async function formatPrBodyAnnotations(
  worktreePath: string,
  prBody: string,
): Promise<string> {
  const virtualPath = `${worktreePath}/__redpen__/pr-body.md`;

  let sidecar;
  try {
    sidecar = await getAnnotations(virtualPath);
  } catch {
    return "";
  }

  if (!sidecar?.annotations.length) return "";

  const blocks = splitMarkdownBlocks(prBody);
  const blockMap = new Map(blocks.map(b => [b.lineNumber, b]));

  const roots = sidecar.annotations
    .filter((a: Annotation) => !a.replyTo)
    .sort((a: Annotation, b: Annotation) =>
      a.anchor.range.startLine - b.anchor.range.startLine
    );

  if (roots.length === 0) return "";

  const replyMap = new Map<string, Annotation[]>();
  for (const ann of sidecar.annotations) {
    if (ann.replyTo) {
      const group = replyMap.get(ann.replyTo) ?? [];
      group.push(ann);
      replyMap.set(ann.replyTo, group);
    }
  }

  const lines: string[] = ["**Comments on PR description:**", ""];

  for (const root of roots) {
    const line = root.anchor.range.startLine;
    const block = blockMap.get(line);
    const preview = block ? blockPreview(block) : `line ${line}`;
    const label = block?.type === "checklist"
      ? `checklist item "${preview}"`
      : `\u00B6${line} (${preview})`;

    lines.push(`> **Re: ${label}:**`);
    lines.push(`> ${root.body}`);

    const replies = replyMap.get(root.id) ?? [];
    for (const reply of replies) {
      lines.push(`> > \u21B3 ${reply.author}: ${reply.body}`);
    }

    lines.push("");
  }

  return lines.join("\n");
}
