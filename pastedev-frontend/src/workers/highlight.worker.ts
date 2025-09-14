import hljs from 'highlight.js';

interface WorkerMessage {
  content: string;
  id: string;
}

interface WorkerResponse {
  id: string;
  success: boolean;
  highlightedCode?: string;
  language?: string;
  error?: string;
}

self.onmessage = (event: MessageEvent<WorkerMessage>) => {
  const { content, id } = event.data;

  try {
    const result = hljs.highlightAuto(content);
    const response: WorkerResponse = {
      id,
      success: true,
      highlightedCode: result.value,
      language: result.language,
    };
    self.postMessage(response);
  } catch (error) {
    const response: WorkerResponse = {
      id,
      success: false,
      error: error instanceof Error ? error.message : 'Unknown error',
    };
    self.postMessage(response);
  }
};