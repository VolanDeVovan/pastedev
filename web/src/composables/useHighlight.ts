import { ref, onUnmounted } from 'vue';

interface Reply {
  id: number;
  html: string;
  language: string | null;
}

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

export function useHighlight() {
  const html = ref('');
  const language = ref<string | null>(null);
  let token = 0;

  const worker = ensureWorker();
  function onMessage(e: MessageEvent<Reply>) {
    if (e.data.id === token) {
      html.value = e.data.html;
      language.value = e.data.language;
    }
  }
  worker.addEventListener('message', onMessage);
  onUnmounted(() => worker.removeEventListener('message', onMessage));

  function highlight(body: string, hint?: string) {
    token++;
    worker.postMessage({ id: token, body, hint });
  }

  return { html, language, highlight };
}
