import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { useKeyboard } from '../hooks/useKeyboard';

interface ViewerProps {
  content: string;
}

const Viewer: React.FC<ViewerProps> = ({ content }) => {
  const codeRef = useRef<HTMLElement>(null);
  const workerRef = useRef<Worker | null>(null);
  const requestIdRef = useRef(0);
  const [highlightedCode, setHighlightedCode] = useState('');
  const lines = content.split('\n');

  useEffect(() => {
    if (!workerRef.current) {
      workerRef.current = new Worker(
        new URL('../workers/highlight.worker.ts', import.meta.url),
      );

      workerRef.current.onmessage = (event) => {
        const { success, highlightedCode: highlighted, error } = event.data;

        if (success) {
          setHighlightedCode(highlighted);
        } else {
          console.error('Highlighting error:', error);
          setHighlightedCode(
            content.replace(/</g, '&lt;').replace(/>/g, '&gt;'),
          );
        }

      };
    }

    return () => {
      if (workerRef.current) {
        workerRef.current.terminate();
        workerRef.current = null;
      }
    };
  }, [content]);

  useEffect(() => {
    if (content && workerRef.current) {
      setHighlightedCode(''); // Clear previous highlighting
      const requestId = ++requestIdRef.current;

      workerRef.current.postMessage({
        content,
        id: requestId,
      });
    }
  }, [content]);

  useKeyboard({
    onEscape: () => {
      window.location.href = '/';
    },
    onSelectAll: () => {
      const selection = window.getSelection();
      if (selection && codeRef.current) {
        const range = document.createRange();
        range.selectNodeContents(codeRef.current);
        selection.removeAllRanges();
        selection.addRange(range);
      }
    },
  });

  return (
    <div className="h-full overflow-auto">
      <div className="flex">
        <div className="bg-gray-900 border-r border-gray-700 min-w-[40px] p-2">
          {lines.map((_, index) => (
            <div
              key={index + 1}
              className="text-gray-500 text-sm text-right leading-relaxed font-mono select-none"
              style={{
                fontFamily:
                  'Fira Code, SF Mono, Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Liberation Mono", Menlo, "Courier New", monospace',
              }}
            >
              {index + 1}
            </div>
          ))}
        </div>

        <div className="flex-1">
          <pre className="p-2 m-0">
            <code
              ref={codeRef}
              className="block text-sm leading-relaxed font-mono text-white"
              style={{
                fontFamily:
                  'Fira Code, SF Mono, Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Liberation Mono", Menlo, "Courier New", monospace',
              }}
              dangerouslySetInnerHTML={{
                __html:
                  highlightedCode ||
                  content.replace(/</g, '&lt;').replace(/>/g, '&gt;'),
              }}
            />
          </pre>
        </div>
      </div>
    </div>
  );
};

export default Viewer;
