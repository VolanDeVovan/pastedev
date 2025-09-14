import React from 'react';
import Editor from './components/Editor';
import Menu from './components/Menu';
import Viewer from './components/Viewer';
import { useSnippetApp } from './hooks/useSnippetApp';

const App = () => {
  const {
    state,
    content,
    setContent,
    snippetData,
    error,
    handleSaveSnippet,
    handleNewSnippet,
    handleEditSnippet,
    getRawUrl,
  } = useSnippetApp();

  return (
    <div className="h-screen bg-gray-800 text-white font-mono relative overflow-hidden">
      {error && (
        <div className="absolute top-4 left-1/2 transform -translate-x-1/2 bg-red-600 text-white px-4 py-2 rounded z-50">
          {error}
        </div>
      )}

      <div 
        className={`absolute top-4 right-4 z-50 ${
          state === 'view' && snippetData?.ephemeral && snippetData?.expiresAt 
            ? 'mt-10' 
            : ''
        }`}
      >
        <Menu
          state={state}
          onSave={state === 'edit' ? () => handleSaveSnippet(content) : undefined}
          onNew={handleNewSnippet}
          onEdit={state === 'view' ? handleEditSnippet : undefined}
          rawUrl={getRawUrl()}
        />
      </div>

      {state === 'loading' && (
        <div className="flex items-center justify-center h-full">
          <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-white"></div>
        </div>
      )}

      {state === 'edit' && (
        <Editor
          content={content}
          onChange={setContent}
          onSave={() => handleSaveSnippet(content)}
        />
      )}

      {state === 'view' && (
        <Viewer
          content={content}
          ephemeral={snippetData?.ephemeral}
          expiresAt={snippetData?.expiresAt}
        />
      )}
    </div>
  );
};

export default App;
