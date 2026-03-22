import type { LanguageSupport } from "@codemirror/language";
import { javascript } from "@codemirror/lang-javascript";
import { python } from "@codemirror/lang-python";
import { rust } from "@codemirror/lang-rust";
import { json } from "@codemirror/lang-json";
import { html } from "@codemirror/lang-html";
import { css } from "@codemirror/lang-css";
import { markdown } from "@codemirror/lang-markdown";
import { java } from "@codemirror/lang-java";
import { cpp } from "@codemirror/lang-cpp";
import { go } from "@codemirror/lang-go";
import { xml } from "@codemirror/lang-xml";
import { sql } from "@codemirror/lang-sql";
import { yaml } from "@codemirror/lang-yaml";

const languageMap: Record<string, () => LanguageSupport> = {
  js: () => javascript(),
  jsx: () => javascript({ jsx: true }),
  ts: () => javascript({ typescript: true }),
  tsx: () => javascript({ jsx: true, typescript: true }),
  py: () => python(),
  rs: () => rust(),
  json: () => json(),
  html: () => html(),
  htm: () => html(),
  css: () => css(),
  md: () => markdown(),
  java: () => java(),
  c: () => cpp(),
  cpp: () => cpp(),
  cc: () => cpp(),
  h: () => cpp(),
  hpp: () => cpp(),
  go: () => go(),
  xml: () => xml(),
  svg: () => xml(),
  sql: () => sql(),
  yaml: () => yaml(),
  yml: () => yaml(),
  svelte: () => html(),
  swift: () => javascript(), // Reasonable fallback — Swift syntax is close enough for highlighting
};

export function getLanguageForExtension(
  ext: string
): LanguageSupport | undefined {
  const factory = languageMap[ext.toLowerCase()];
  return factory ? factory() : undefined;
}
