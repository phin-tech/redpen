import { Marked } from "marked";
import type { Tokens, RendererObject } from "marked";
import DOMPurify from "dompurify";

let currentSource = "";

function findSourceLine(raw: string): number | null {
  if (!currentSource || !raw) return null;
  const trimmed = raw.trim();
  const idx = currentSource.indexOf(trimmed);
  if (idx === -1) return null;
  let line = 1;
  for (let i = 0; i < idx; i++) {
    if (currentSource[i] === "\n") line++;
  }
  return line;
}

function attr(raw: string): string {
  const line = findSourceLine(raw);
  return line !== null ? ` data-source-line="${line}"` : "";
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

// Custom renderer that adds data-source-line attributes.
// Uses regular functions (not arrows) so `this.parser` is available
// for rendering inline tokens.
const renderer: RendererObject = {
  heading(this: any, token: Tokens.Heading) {
    const text = this.parser.parseInline(token.tokens);
    return `<h${token.depth}${attr(token.raw)}>${text}</h${token.depth}>\n`;
  },
  paragraph(this: any, token: Tokens.Paragraph) {
    const text = this.parser.parseInline(token.tokens);
    return `<p${attr(token.raw)}>${text}</p>\n`;
  },
  code(token: Tokens.Code) {
    if (token.lang === "mermaid") {
      return `<pre class="mermaid"${attr(token.raw)}>${escapeHtml(token.text)}</pre>\n`;
    }
    const langClass = token.lang ? ` class="language-${escapeHtml(token.lang)}"` : "";
    return `<pre${attr(token.raw)}><code${langClass}>${escapeHtml(token.text)}</code></pre>\n`;
  },
  blockquote(this: any, token: Tokens.Blockquote) {
    const body = this.parser.parse(token.tokens);
    return `<blockquote${attr(token.raw)}>${body}</blockquote>\n`;
  },
  table(this: any, token: Tokens.Table) {
    // Render header
    let header = "<tr>";
    for (const cell of token.header) {
      const content = this.parser.parseInline(cell.tokens);
      const align = cell.align ? ` style="text-align:${cell.align}"` : "";
      header += `<th${align}>${content}</th>`;
    }
    header += "</tr>";

    // Render rows
    let rows = "";
    for (const row of token.rows) {
      rows += "<tr>";
      for (const cell of row) {
        const content = this.parser.parseInline(cell.tokens);
        const align = cell.align ? ` style="text-align:${cell.align}"` : "";
        rows += `<td${align}>${content}</td>`;
      }
      rows += "</tr>";
    }

    return `<table${attr(token.raw)}><thead>${header}</thead><tbody>${rows}</tbody></table>\n`;
  },
  list(this: any, token: Tokens.List) {
    let body = "";
    for (const item of token.items) {
      body += this.listitem(item);
    }
    const tag = token.ordered ? "ol" : "ul";
    const startAttr = token.ordered && token.start !== 1 ? ` start="${token.start}"` : "";
    return `<${tag}${startAttr}${attr(token.raw)}>${body}</${tag}>\n`;
  },
  listitem(this: any, token: Tokens.ListItem) {
    let text = "";
    if (token.tokens) {
      text = this.parser.parse(token.tokens, !!token.loose);
    }
    return `<li${attr(token.raw)}>${text}</li>\n`;
  },
  hr(token: Tokens.Hr) {
    return `<hr${attr(token.raw)} />\n`;
  },
};

const marked = new Marked({ renderer });

export function renderMarkdown(source: string): string {
  currentSource = source;
  const raw = marked.parse(source) as string;
  currentSource = "";

  const clean = DOMPurify.sanitize(raw, {
    ADD_ATTR: ["data-source-line"],
    ADD_TAGS: ["pre"],
  });

  return clean;
}
