import { createHighlighter } from "shiki/bundle/web";

let highlighter: any = null;

// Initialize highlighter
async function initHighlighter() {
  if (!highlighter) {
    highlighter = await createHighlighter({
      themes: ["github-dark"],
      langs: [
        "angular-html",
        "angular-ts",
        "astro",
        "blade",
        "coffee",
        "css",
        "graphql",
        "haml",
        "handlebars",
        "html",
        "html-derivative",
        "http",
        "imba",
        "javascript",
        "jinja",
        "jison",
        "json",
        "json5",
        "jsonc",
        "jsonl",
        "jsx",
        "julia",
        "less",
        "markdown",
        "marko",
        "mdc",
        "mdx",
        "php",
        "postcss",
        "pug",
        "sass",
        "scss",
        "stylus",
        "svelte",
        "ts-tags",
        "tsx",
        "typescript",
        "vue",
        "vue-html",
        "vue-vine",
        "wasm",
        "wgsl",
        "wit",
        "text",
      ],
    });
  }
  return highlighter;
}

// Message types
interface HighlightRequest {
  type: "highlight";
  id: string;
  code: string;
  language?: string;
}

interface HighlightResponse {
  type: "highlighted";
  id: string;
  lines: Array<{
    lineNumber: number;
    content: string;
  }>;
}

interface ErrorResponse {
  type: "error";
  id: string;
  error: string;
}

self.onmessage = async (event: MessageEvent<HighlightRequest>) => {
  const { type, id, code, language } = event.data;

  if (type === "highlight") {
    try {
      const hl = await initHighlighter();

      // Always use text for now
      const lang = "text";

      // Split code into lines for virtual scrolling
      const codeLines = code.split("\n");
      const lines: Array<{ lineNumber: number; content: string }> = [];

      // Process each line
      for (let i = 0; i < codeLines.length; i++) {
        const line = codeLines[i];

        if (line.trim() === "") {
          // Empty line
          lines.push({
            lineNumber: i + 1,
            content: '<span class="line"></span>',
          });
        } else {
          try {
            // Try to highlight with detected/specified language
            const highlighted = hl.codeToHtml(line, {
              lang,
              theme: "github-dark",
              structure: "inline",
            });

            // Extract the content inside the code tag
            const match = highlighted.match(/<code[^>]*>(.*?)<\/code>/s);
            const content = match ? match[1] : line;

            lines.push({
              lineNumber: i + 1,
              content: `<span class="line">${content}</span>`,
            });
          } catch (lineError) {
            // If highlighting fails, fallback to 'text' language
            try {
              const plainHighlighted = hl.codeToHtml(line, {
                lang: "text",
                theme: "github-dark",
                structure: "inline",
              });
              const match = plainHighlighted.match(/<code[^>]*>(.*?)<\/code>/s);
              const content = match ? match[1] : line;

              lines.push({
                lineNumber: i + 1,
                content: `<span class="line">${content}</span>`,
              });
            } catch {
              // Final fallback - just escape HTML
              const escapedLine = line
                .replace(/&/g, "&amp;")
                .replace(/</g, "&lt;")
                .replace(/>/g, "&gt;");

              lines.push({
                lineNumber: i + 1,
                content: `<span class="line">${escapedLine}</span>`,
              });
            }
          }
        }
      }

      const response: HighlightResponse = {
        type: "highlighted",
        id,
        lines,
      };

      self.postMessage(response);
    } catch (error) {
      console.error("Worker error:", error);
      const errorResponse: ErrorResponse = {
        type: "error",
        id,
        error: error instanceof Error ? error.message : "Unknown error",
      };

      self.postMessage(errorResponse);
    }
  }
};
