/// <reference lib="webworker" />

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
}

self.onmessage = (e: MessageEvent<Request>) => {
  const { id, body, hint } = e.data;
  const result = hint && hljs.getLanguage(hint)
    ? hljs.highlight(body, { language: hint, ignoreIllegals: true })
    : hljs.highlightAuto(body);
  const reply: Reply = {
    id,
    html: result.value,
    language: result.language ?? null,
  };
  (self as unknown as Worker).postMessage(reply);
};
