// hljs Web Worker — same shape as the Vue version at
// web/src/workers/highlight.worker.ts. Receives { id, body, hint? } and
// replies with { id, html, language, truncated }. The Rust caller treats
// stale replies (lower id than the current request) as ignorable.
//
// hljs.highlightAuto scores every registered language on the body, which is
// quadratic-ish in size. For large pastes we sample-detect on the head and
// then run the linear-time language-specific path on the full body.

self.importScripts('/assets/highlight.min.js');

const DETECT_SAMPLE_BYTES = 16 * 1024;
const HARD_LIMIT = 4 * 1024 * 1024;

function escapeHtml(s) {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

self.onmessage = (e) => {
  const { id, body, hint } = e.data;
  const len = body.length;

  if (len > HARD_LIMIT) {
    self.postMessage({ id, html: escapeHtml(body), language: null, truncated: true });
    return;
  }

  try {
    // Path 1 — explicit hint we trust hljs recognises.
    if (hint && self.hljs.getLanguage(hint)) {
      const r = self.hljs.highlight(body, { language: hint, ignoreIllegals: true });
      self.postMessage({ id, html: r.value, language: r.language || hint, truncated: false });
      return;
    }
    // Path 2 — small body, full auto-detect.
    if (len <= DETECT_SAMPLE_BYTES) {
      const r = self.hljs.highlightAuto(body);
      self.postMessage({ id, html: r.value, language: r.language || null, truncated: false });
      return;
    }
    // Path 3 — large body, sample-detect then run the explicit path.
    const sample = body.slice(0, DETECT_SAMPLE_BYTES);
    const detected = self.hljs.highlightAuto(sample);
    const lang = detected.language;
    if (!lang || !self.hljs.getLanguage(lang)) {
      self.postMessage({ id, html: escapeHtml(body), language: null, truncated: false });
      return;
    }
    const r = self.hljs.highlight(body, { language: lang, ignoreIllegals: true });
    self.postMessage({ id, html: r.value, language: r.language || lang, truncated: false });
  } catch (_err) {
    // Defensive: never let the worker die. Echo escaped text on any failure.
    self.postMessage({ id, html: escapeHtml(body), language: null, truncated: true });
  }
};
