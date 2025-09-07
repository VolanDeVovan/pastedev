export interface HighlightedLine {
  lineNumber: number;
  content: string;
}

export interface HighlightResult {
  id: string;
  lines: HighlightedLine[];
}

export class SyntaxHighlighter {
  private worker: Worker | null = null;
  private requestId = 0;
  private pendingRequests = new Map<string, {
    resolve: (result: HighlightResult) => void;
    reject: (error: Error) => void;
  }>();

  constructor() {
    this.initWorker();
  }

  private initWorker() {
    try {
      // Create worker from the TypeScript file - Vite will handle the bundling
      this.worker = new Worker(
        new URL('../workers/syntax-highlighter.worker.ts', import.meta.url),
        { type: 'module' }
      );

      this.worker.onmessage = (event) => {
        const { type, id, lines, error } = event.data;
        
        const request = this.pendingRequests.get(id);
        if (!request) return;
        
        this.pendingRequests.delete(id);
        
        if (type === 'highlighted') {
          request.resolve({ id, lines });
        } else if (type === 'error') {
          request.reject(new Error(error));
        }
      };

      this.worker.onerror = (error) => {
        console.error('Worker error:', error);
        // Reject all pending requests
        for (const [id, request] of this.pendingRequests) {
          request.reject(new Error('Worker error'));
        }
        this.pendingRequests.clear();
      };
    } catch (error) {
      console.error('Failed to initialize syntax highlighter worker:', error);
    }
  }

  async highlight(code: string, language?: string): Promise<HighlightResult> {
    if (!this.worker) {
      // Fallback: return unhighlighted lines
      const lines = code.split('\n').map((line, index) => ({
        lineNumber: index + 1,
        content: `<span class="line">${line}</span>`
      }));
      return { id: 'fallback', lines };
    }

    const id = `highlight_${++this.requestId}`;
    
    return new Promise((resolve, reject) => {
      this.pendingRequests.set(id, { resolve, reject });
      
      this.worker!.postMessage({
        type: 'highlight',
        id,
        code,
        language
      });
      
      // Timeout after 10 seconds
      setTimeout(() => {
        if (this.pendingRequests.has(id)) {
          this.pendingRequests.delete(id);
          reject(new Error('Syntax highlighting timeout'));
        }
      }, 10000);
    });
  }

  destroy() {
    if (this.worker) {
      this.worker.terminate();
      this.worker = null;
    }
    this.pendingRequests.clear();
  }
}