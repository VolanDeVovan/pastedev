import MarkdownIt from 'markdown-it';
import hljs from 'highlight.js';

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;');
}

function highlightFence(str: string, lang: string): string {
  if (lang && hljs.getLanguage(lang)) {
    try {
      const out = hljs.highlight(str, { language: lang, ignoreIllegals: true }).value;
      return `<pre><code class="hljs language-${lang}">${out}</code></pre>`;
    } catch (_) {
      // fall through to plain escaped output
    }
  }
  return `<pre><code class="hljs">${escapeHtml(str)}</code></pre>`;
}

// markdown-it's default validateLink already rejects javascript:/vbscript:/file:
// and data: (except images). We do NOT override it — that's a known XSS hole.
const md = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  breaks: false,
  highlight: highlightFence,
});

export function renderMarkdown(src: string): string {
  return md.render(src);
}
