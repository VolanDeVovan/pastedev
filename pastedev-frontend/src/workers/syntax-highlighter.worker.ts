import { createHighlighter } from "shiki/bundle/web";

let highlighter: any = null;

async function initHighlighter() {
  if (!highlighter) {
    highlighter = await createHighlighter({
      themes: ["github-dark"],
      langs: [
        "typescript",
        "javascript",
        "json",
        "html",
        "css",
        "markdown",
        "text",
      ],
    });
  }
  return highlighter;
}

self.onmessage = async (event) => {
  const { id, code, language } = event.data;

  try {
    const hl = await initHighlighter();
    const html = hl.codeToHtml(code, { lang: language, theme: "github-dark" });
    const content = html.match(/<code[^>]*>(.*?)<\/code>/s)?.[1] || code;

    const lines = content.split("\n").map((line, index) => ({
      lineNumber: index + 1,
      content: line || '<span class="line"></span>',
    }));

    self.postMessage({ id, lines });
  } catch {
    const lines = code.split("\n").map((line, index) => ({
      lineNumber: index + 1,
      content: line.replace(
        /[&<>]/g,
        (m) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[m]!,
      ),
    }));

    self.postMessage({ id, lines });
  }
};
