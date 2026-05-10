/// <reference lib="webworker" />

// highlight.js auto-detection is the expensive operation — quadratic-ish in
// input size because it scores every registered language. The trick to keep
// it fast on large pastes: detect on a sample, then highlight the full body
// with the explicit language. `highlight(body, {language})` is linear in body
// size and stays fast well past a megabyte.

import hljs from 'highlight.js';

interface Request {
  id: number;
  body: string;
  hint?: string;
}

interface Reply {
  id: number;
  html: string;
  language: string | null;
  truncated: boolean;
}

// Size of the sample taken for auto-detection. Big enough to give hljs signal
// (function bodies, common idioms, comments) but small enough to detect in
// well under 100ms.
const DETECT_SAMPLE_BYTES = 16 * 1024;

// Above this body length we bail entirely — for context, the server caps the
// snippet at 1 MB, so this only kicks in for editor previews where the user
// hasn't published yet.
const HARD_LIMIT = 4 * 1024 * 1024;

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

self.onmessage = (e: MessageEvent<Request>) => {
  const { id, body, hint } = e.data;
  const len = body.length;

  if (len > HARD_LIMIT) {
    (self as unknown as Worker).postMessage({
      id,
      html: escapeHtml(body),
      language: null,
      truncated: true,
    } satisfies Reply);
    return;
  }

  try {
    // Path 1 — explicit hint. Linear-time render with the given language.
    if (hint && hljs.getLanguage(hint)) {
      const r = hljs.highlight(body, { language: hint, ignoreIllegals: true });
      (self as unknown as Worker).postMessage({
        id,
        html: r.value,
        language: r.language ?? hint,
        truncated: false,
      } satisfies Reply);
      return;
    }

    // Path 2 — small body, auto-detect the whole thing.
    if (len <= DETECT_SAMPLE_BYTES) {
      const r = hljs.highlightAuto(body);
      (self as unknown as Worker).postMessage({
        id,
        html: r.value,
        language: r.language ?? null,
        truncated: false,
      } satisfies Reply);
      return;
    }

    // Path 3 — large body, sample-detect, then full render with the detected
    // language. We sample from the start; for source files the first 16KB is
    // almost always representative.
    const sample = body.slice(0, DETECT_SAMPLE_BYTES);
    const detected = hljs.highlightAuto(sample);
    const lang = detected.language;
    if (!lang || !hljs.getLanguage(lang)) {
      // Couldn't detect a language. Fall back to escaped plain text rather than
      // mis-highlight the whole file.
      (self as unknown as Worker).postMessage({
        id,
        html: escapeHtml(body),
        language: null,
        truncated: false,
      } satisfies Reply);
      return;
    }
    const r = hljs.highlight(body, { language: lang, ignoreIllegals: true });
    (self as unknown as Worker).postMessage({
      id,
      html: r.value,
      language: r.language ?? lang,
      truncated: false,
    } satisfies Reply);
  } catch (err) {
    // Defensive: never let the worker die. Echo escaped text on any failure.
    (self as unknown as Worker).postMessage({
      id,
      html: escapeHtml(body),
      language: null,
      truncated: true,
    } satisfies Reply);
  }
};
