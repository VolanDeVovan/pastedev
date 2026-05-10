import MarkdownIt from 'markdown-it';

// markdown-it's default validateLink already rejects javascript:/vbscript:/file:
// and data: (except images). We do NOT override it — that's a known XSS hole.
const md = new MarkdownIt({
  html: false,
  linkify: true,
  typographer: true,
  breaks: false,
});

export function renderMarkdown(src: string): string {
  return md.render(src);
}
