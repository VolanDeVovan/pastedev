import { ref, onUnmounted } from 'vue';

interface Reply {
  id: number;
  html: string;
  language: string | null;
  truncated: boolean;
}

// One Worker per tab, shared across views. Importing the file with the
// `?worker` query is the Vite-native way to spawn a Worker bundle.
let workerRef: Worker | null = null;

function ensureWorker(): Worker {
  if (workerRef === null) {
    workerRef = new Worker(
      new URL('../workers/highlight.worker.ts', import.meta.url),
      { type: 'module' },
    );
  }
  return workerRef;
}

// Above this body length we skip the worker round-trip entirely and render
// escaped plain text immediately. Aligned with the server's 1 MB snippet cap —
// nothing larger than this can be published, so for the published view we'll
// never hit the skip path. Editor pastes >1MB get plain text + the size
// counter goes red.
const SKIP_WORKER_BYTES = 1_100_000;

const DEBOUNCE_MS = 150;

export function useHighlight() {
  const html = ref('');
  const language = ref<string | null>(null);
  const truncated = ref(false);
  let token = 0;
  let pending: number | null = null;

  const worker = ensureWorker();
  function onMessage(e: MessageEvent<Reply>) {
    if (e.data.id === token) {
      html.value = e.data.html;
      language.value = e.data.language;
      truncated.value = e.data.truncated;
    }
  }
  worker.addEventListener('message', onMessage);
  onUnmounted(() => worker.removeEventListener('message', onMessage));

  function escapeHtml(s: string): string {
    return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }

  function highlight(body: string, hint?: string) {
    token++;
    const t = token;
    if (pending !== null) {
      clearTimeout(pending);
      pending = null;
    }
    // Fast path for huge bodies — don't even cross the worker boundary.
    // Render escaped plain text immediately.
    if (body.length > SKIP_WORKER_BYTES) {
      html.value = escapeHtml(body);
      language.value = null;
      truncated.value = true;
      return;
    }
    pending = window.setTimeout(() => {
      pending = null;
      // If a newer call landed before our timeout fired, our token is stale
      // and the worker reply will be ignored — but we still need to send a
      // message so the worker stays warm.
      worker.postMessage({ id: t, body, hint });
    }, DEBOUNCE_MS);
  }

  return { html, language, truncated, highlight };
}
