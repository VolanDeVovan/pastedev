export interface HighlightedLine {
  lineNumber: number;
  content: string;
}

export class SyntaxHighlighter {
  private worker: Worker | null = null;
  private requestId = 0;
  private pendingRequests = new Map<
    string,
    (lines: HighlightedLine[]) => void
  >();

  constructor() {
    this.initWorker();
  }

  private initWorker() {
    try {
      this.worker = new Worker(
        new URL("../workers/syntax-highlighter.worker.ts", import.meta.url),
        { type: "module" },
      );

      this.worker.onmessage = (event) => {
        const { id, lines } = event.data;
        const resolve = this.pendingRequests.get(id);
        if (resolve) {
          this.pendingRequests.delete(id);
          resolve(lines);
        }
      };
    } catch (error) {
      console.error("Failed to initialize syntax highlighter worker:", error);
    }
  }

  async highlight(
    code: string,
    language = "typescript",
  ): Promise<HighlightedLine[]> {
    if (!this.worker) {
      return code.split("\n").map((line, index) => ({
        lineNumber: index + 1,
        content: line.replace(
          /[&<>]/g,
          (m) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[m]!,
        ),
      }));
    }

    const id = `${++this.requestId}`;

    return new Promise((resolve) => {
      this.pendingRequests.set(id, resolve);
      this.worker!.postMessage({ id, code, language });

      setTimeout(() => {
        if (this.pendingRequests.delete(id)) {
          resolve(
            code.split("\n").map((line, index) => ({
              lineNumber: index + 1,
              content: line.replace(
                /[&<>]/g,
                (m) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[m]!,
              ),
            })),
          );
        }
      }, 5000);
    });
  }

  destroy() {
    this.worker?.terminate();
    this.pendingRequests.clear();
  }
}
