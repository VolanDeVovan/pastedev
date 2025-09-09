import { createHighlighter } from "shiki/bundle/web";
import hljs from "highlight.js";
import { mapHljsLanguageToShiki } from "../utils/language-mapper.js";

let highlighter: any = null;

async function initHighlighter() {
  if (!highlighter) {
    highlighter = await createHighlighter({
      themes: ["github-dark"],
      langs: [
        // Web-only supported languages from Shiki
        "angular-html",
        "angular-ts",
        "astro",
        "blade",
        "c",
        "cpp",
        "coffeescript",
        "css",
        "glsl",
        "graphql",
        "haml",
        "handlebars",
        "html",
        "http",
        "imba",
        "java",
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
        "python",
        "r",
        "regexp",
        "sass",
        "scss",
        "shellscript",
        "sql",
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
        "xml",
        "yaml",
        "text",
      ],
    });
  }
  return highlighter;
}


self.onmessage = async (event) => {
  const { id, code } = event.data;

  try {
    // Detect language using highlight.js
    const detection = hljs.highlightAuto(code);
    const detectedLanguage = mapHljsLanguageToShiki(detection.language || "text");

    const hl = await initHighlighter();

    // Check if the detected language is supported by our shiki instance
    const loadedLanguages = hl.getLoadedLanguages();
    const langToUse = loadedLanguages.includes(detectedLanguage) ? detectedLanguage : "text";

    const html = hl.codeToHtml(code, { lang: langToUse, theme: "github-dark" });
    const content = html.match(/<code[^>]*>(.*?)<\/code>/s)?.[1] || code;

    const lines = content.split("\n").map((line: string, index: number) => ({
      lineNumber: index + 1,
      content: line || '<span class="line"></span>',
    }));

    self.postMessage({ id, lines });
  } catch (error) {
    console.error("Highlighting error:", error);
    const lines = code.split("\n").map((line: string, index: number) => ({
      lineNumber: index + 1,
      content: line.replace(
        /[&<>]/g,
        (m: string) => {
          const entityMap: { [key: string]: string } = { "&": "&amp;", "<": "&lt;", ">": "&gt;" };
          return entityMap[m];
        },
      ),
    }));

    self.postMessage({ id, lines });
  }
};
