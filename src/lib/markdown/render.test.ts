import { describe, expect, it } from "vitest";
import { renderMarkdown } from "./render";

describe("renderMarkdown", () => {
  it("renders a heading", () => {
    const html = renderMarkdown("# Hello World");
    expect(html).toContain("<h1");
    expect(html).toContain("Hello World");
    expect(html).toContain("</h1>");
  });

  it("renders a paragraph", () => {
    const html = renderMarkdown("Some text here.");
    expect(html).toContain("<p");
    expect(html).toContain("Some text here.");
    expect(html).toContain("</p>");
  });

  it("renders inline code", () => {
    const html = renderMarkdown("Use `foo()` here.");
    expect(html).toContain("<code>");
    expect(html).toContain("foo()");
  });

  it("renders fenced code blocks", () => {
    const html = renderMarkdown("```js\nconst x = 1;\n```");
    expect(html).toContain("<pre");
    expect(html).toContain('<code class="language-js"');
    expect(html).toContain("const x = 1;");
  });

  it("renders mermaid blocks with pre.mermaid", () => {
    const html = renderMarkdown("```mermaid\ngraph TD;\n  A-->B;\n```");
    expect(html).toContain('<pre class="mermaid"');
    expect(html).toContain("graph TD;");
    // Should NOT wrap in <code>
    expect(html).not.toContain("<code");
  });

  it("renders blockquotes", () => {
    const html = renderMarkdown("> A quote");
    expect(html).toContain("<blockquote");
    expect(html).toContain("A quote");
  });

  it("renders unordered lists", () => {
    const html = renderMarkdown("- one\n- two\n- three");
    expect(html).toContain("<ul");
    expect(html).toContain("<li");
    expect(html).toContain("one");
    expect(html).toContain("three");
  });

  it("renders ordered lists", () => {
    const html = renderMarkdown("1. first\n2. second");
    expect(html).toContain("<ol");
    expect(html).toContain("<li");
    expect(html).toContain("first");
  });

  it("renders tables", () => {
    const md = "| A | B |\n|---|---|\n| 1 | 2 |";
    const html = renderMarkdown(md);
    expect(html).toContain("<table");
    expect(html).toContain("<thead");
    expect(html).toContain("<tbody");
    expect(html).toContain("<th");
    expect(html).toContain("<td");
  });

  it("renders horizontal rules", () => {
    const html = renderMarkdown("---");
    expect(html).toContain("<hr");
  });

  it("renders links", () => {
    const html = renderMarkdown("[click](https://example.com)");
    expect(html).toContain("<a");
    expect(html).toContain('href="https://example.com"');
    expect(html).toContain("click");
  });

  it("renders images", () => {
    const html = renderMarkdown("![alt](image.png)");
    expect(html).toContain("<img");
    expect(html).toContain('src="image.png"');
    expect(html).toContain('alt="alt"');
  });

  it("renders bold and italic", () => {
    const html = renderMarkdown("**bold** and *italic*");
    expect(html).toContain("<strong>bold</strong>");
    expect(html).toContain("<em>italic</em>");
  });

  describe("source line mapping", () => {
    it("adds data-source-line to headings", () => {
      const html = renderMarkdown("# Title\n\nParagraph");
      expect(html).toMatch(/data-source-line="1"/);
    });

    it("adds data-source-line to paragraphs", () => {
      const html = renderMarkdown("# Title\n\nSome paragraph text");
      // The paragraph starts on line 3
      expect(html).toMatch(/<p[^>]*data-source-line="3"/);
    });

    it("adds data-source-line to code blocks", () => {
      const md = "text\n\n```js\ncode\n```";
      const html = renderMarkdown(md);
      expect(html).toMatch(/<pre[^>]*data-source-line="3"/);
    });

    it("adds data-source-line to mermaid blocks", () => {
      const md = "intro\n\n```mermaid\ngraph TD;\n```";
      const html = renderMarkdown(md);
      expect(html).toMatch(/<pre class="mermaid"[^>]*data-source-line="3"/);
    });

    it("adds data-source-line to list items", () => {
      const md = "intro\n\n- item one\n- item two";
      const html = renderMarkdown(md);
      expect(html).toMatch(/<li[^>]*data-source-line/);
    });

    it("adds data-source-line to blockquotes", () => {
      const md = "intro\n\n> quoted text";
      const html = renderMarkdown(md);
      expect(html).toMatch(/<blockquote[^>]*data-source-line="3"/);
    });
  });

  describe("sanitization", () => {
    it("strips script tags", () => {
      const html = renderMarkdown('<script>alert("xss")</script>');
      expect(html).not.toContain("<script");
      expect(html).not.toContain("alert");
    });

    it("strips onerror attributes", () => {
      const html = renderMarkdown('<img src="x" onerror="alert(1)">');
      expect(html).not.toContain("onerror");
    });

    it("preserves data-source-line attributes", () => {
      const html = renderMarkdown("# Heading");
      expect(html).toContain("data-source-line");
    });
  });
});
