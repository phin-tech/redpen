export interface MarkdownBlock {
  lineNumber: number;
  content: string;
  type:
    | "heading"
    | "paragraph"
    | "checklist"
    | "listItem"
    | "codeBlock"
    | "blockquote"
    | "other";
}

const SEPARATOR_RE = /^(---+|\*\*\*+|___+)\s*$/;
const HEADING_RE = /^#{1,6}\s/;
const CHECKLIST_RE = /^[-*]\s+\[[ xX]\]\s/;
const UNORDERED_RE = /^[-*+]\s/;
const ORDERED_RE = /^\d+\.\s/;
const CODE_FENCE_RE = /^```/;
const BLOCKQUOTE_RE = /^>\s?/;

export function splitMarkdownBlocks(body: string): MarkdownBlock[] {
  const lines = body.split("\n");
  const blocks: MarkdownBlock[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    // Skip blank lines
    if (line.trim() === "") {
      i++;
      continue;
    }

    // Skip separators
    if (SEPARATOR_RE.test(line.trim())) {
      i++;
      continue;
    }

    // Code block (fenced)
    if (CODE_FENCE_RE.test(line.trim())) {
      const startLine = i;
      const collected = [line];
      i++;
      while (i < lines.length) {
        collected.push(lines[i]);
        if (i > startLine && CODE_FENCE_RE.test(lines[i].trim())) {
          i++;
          break;
        }
        i++;
      }
      blocks.push({
        lineNumber: startLine + 1,
        content: collected.join("\n"),
        type: "codeBlock",
      });
      continue;
    }

    // Heading
    if (HEADING_RE.test(line)) {
      blocks.push({ lineNumber: i + 1, content: line, type: "heading" });
      i++;
      continue;
    }

    // Checklist item
    if (CHECKLIST_RE.test(line.trim())) {
      blocks.push({ lineNumber: i + 1, content: line, type: "checklist" });
      i++;
      continue;
    }

    // List item (unordered or ordered)
    if (UNORDERED_RE.test(line.trim()) || ORDERED_RE.test(line.trim())) {
      blocks.push({ lineNumber: i + 1, content: line, type: "listItem" });
      i++;
      continue;
    }

    // Blockquote (consume consecutive `>` lines)
    if (BLOCKQUOTE_RE.test(line)) {
      const startLine = i;
      const collected = [line];
      i++;
      while (i < lines.length && BLOCKQUOTE_RE.test(lines[i])) {
        collected.push(lines[i]);
        i++;
      }
      blocks.push({
        lineNumber: startLine + 1,
        content: collected.join("\n"),
        type: "blockquote",
      });
      continue;
    }

    // Paragraph: consume consecutive non-blank, non-special lines
    {
      const startLine = i;
      const collected = [line];
      i++;
      while (
        i < lines.length &&
        lines[i].trim() !== "" &&
        !SEPARATOR_RE.test(lines[i].trim()) &&
        !HEADING_RE.test(lines[i]) &&
        !CODE_FENCE_RE.test(lines[i].trim()) &&
        !CHECKLIST_RE.test(lines[i].trim()) &&
        !UNORDERED_RE.test(lines[i].trim()) &&
        !ORDERED_RE.test(lines[i].trim()) &&
        !BLOCKQUOTE_RE.test(lines[i])
      ) {
        collected.push(lines[i]);
        i++;
      }
      blocks.push({
        lineNumber: startLine + 1,
        content: collected.join("\n"),
        type: "paragraph",
      });
    }
  }

  return blocks;
}

export function blockPreview(block: MarkdownBlock, maxLength = 60): string {
  let text = block.content;

  // Strip heading markers
  text = text.replace(/^#{1,6}\s+/, "");
  // Strip checklist markers
  text = text.replace(/^[-*]\s+\[[ xX]\]\s+/, "");
  // Strip list bullets
  text = text.replace(/^[-*+]\s+/, "");
  text = text.replace(/^\d+\.\s+/, "");
  // Strip blockquote markers
  text = text.replace(/^>\s?/gm, "");
  // Strip bold/italic
  text = text.replace(/\*\*(.+?)\*\*/g, "$1");
  text = text.replace(/\*(.+?)\*/g, "$1");
  text = text.replace(/__(.+?)__/g, "$1");
  text = text.replace(/_(.+?)_/g, "$1");
  // Strip inline code
  text = text.replace(/`(.+?)`/g, "$1");
  // Strip links, keep text
  text = text.replace(/\[(.+?)\]\(.+?\)/g, "$1");
  // Collapse whitespace
  text = text.replace(/\s+/g, " ").trim();

  if (text.length > maxLength) {
    return text.slice(0, maxLength).trimEnd() + "...";
  }
  return text;
}
