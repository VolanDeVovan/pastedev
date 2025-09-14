import { useEffect } from 'react';

export const useKeyboard = (handlers: {
  onSave?: () => void;
  onEscape?: () => void;
  onSelectAll?: () => void;
}) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && handlers.onEscape) {
        handlers.onEscape();
      }

      if (
        (e.ctrlKey || e.metaKey) &&
        (e.key === 'Enter' || e.key === 's') &&
        handlers.onSave
      ) {
        e.preventDefault();
        handlers.onSave();
      }

      if ((e.ctrlKey || e.metaKey) && e.key === 'a' && handlers.onSelectAll) {
        e.preventDefault();
        handlers.onSelectAll();
      }
    };

    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [handlers]);
};
