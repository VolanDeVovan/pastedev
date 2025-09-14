import type React from 'react';
import { useEffect, useRef } from 'react';
import { useKeyboard } from '../hooks/useKeyboard';

interface EditorProps {
  content: string;
  onChange: (content: string) => void;
  onSave: () => void;
}

const Editor: React.FC<EditorProps> = ({ content, onChange, onSave }) => {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.focus();
    }
  }, []);

  useKeyboard({
    onSave,
    onEscape: () => {
      window.location.href = '/';
    },
  });

  return (
    <div className="h-full relative">
      <div className="absolute top-3.5 left-4 text-gray-400 text-lg z-10 pointer-events-none">
        &gt;
      </div>

      <textarea
        ref={textareaRef}
        value={content}
        onChange={(e) => onChange(e.target.value)}
        className="w-full h-full bg-transparent text-white resize-none outline-none border-none p-4 pt-4 pl-8 font-mono text-sm leading-relaxed"
        placeholder="Paste your code, text, or any content here..."
        style={{
          fontFamily:
            'Fira Code, SF Mono, Monaco, "Cascadia Code", "Roboto Mono", Consolas, "Liberation Mono", Menlo, "Courier New", monospace',
        }}
      />
    </div>
  );
};

export default Editor;
