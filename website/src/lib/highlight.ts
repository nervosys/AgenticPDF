import { createHighlighter, type Highlighter } from "shiki";

const globalForShiki = globalThis as typeof globalThis & {
  __shikiHighlighter?: Promise<Highlighter>;
};

function getHighlighter(): Promise<Highlighter> {
  if (!globalForShiki.__shikiHighlighter) {
    globalForShiki.__shikiHighlighter = createHighlighter({
      themes: ["github-dark", "github-light"],
      langs: ["typescript", "javascript", "bash", "html", "json", "css", "markdown", "rust"],
    });
  }
  return globalForShiki.__shikiHighlighter;
}

export async function highlight(code: string, lang = "typescript"): Promise<string> {
  const h = await getHighlighter();
  return h.codeToHtml(code, {
    lang,
    themes: { dark: "github-dark", light: "github-light" },
    defaultColor: false,
  });
}