import { useEffect, useState } from 'react';
import { API_URL } from '../constants';
import { snippetService, type SnippetData } from '../services/snippetService';

export type AppState = 'edit' | 'view' | 'loading';

export const useSnippetApp = () => {
  const [state, setState] = useState<AppState>('edit');
  const [content, setContent] = useState('');
  const [snippetId, setSnippetId] = useState<string | null>(null);
  const [snippetData, setSnippetData] = useState<SnippetData | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const path = window.location.pathname;
    if (path !== '/') {
      const id = path.substring(1);
      setSnippetId(id);
      handleFetchSnippet(id);
    }
  }, []);

  const handleFetchSnippet = async (id: string) => {
    setState('loading');
    try {
      const fetchedSnippet = await snippetService.fetchSnippet(id);
      setContent(fetchedSnippet.content);
      setSnippetData(fetchedSnippet);
      setState('view');
    } catch (err) {
      if (err instanceof Error && err.message === 'SNIPPET_NOT_FOUND') {
        setError('Snippet not found');
        setTimeout(() => {
          setError(null);
          setState('edit');
          setSnippetId(null);
          setSnippetData(null);
          window.history.pushState(null, '', '/');
        }, 3000);
      } else {
        setError(
          err instanceof Error ? err.message : 'Failed to fetch snippet',
        );
        setState('edit');
      }
    }
  };

  const handleSaveSnippet = async (text: string) => {
    setState('loading');
    try {
      const id = await snippetService.saveSnippet(text);
      setSnippetId(id);
      setContent(text);
      setSnippetData(null);
      window.history.pushState(null, '', `/${id}`);
      setState('view');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to save snippet');
      setState('edit');
    }
  };

  const handleNewSnippet = () => {
    setContent('');
    setSnippetId(null);
    setSnippetData(null);
    setState('edit');
    setError(null);
    window.history.pushState(null, '', '/');
  };

  const handleEditSnippet = () => {
    setState('edit');
    setError(null);
  };

  const getRawUrl = () => {
    if (snippetId) {
      return `${API_URL}/api/snippets/${snippetId}`;
    }
    return null;
  };

  return {
    state,
    content,
    setContent,
    snippetId,
    snippetData,
    error,
    handleSaveSnippet,
    handleNewSnippet,
    handleEditSnippet,
    getRawUrl,
  };
};
