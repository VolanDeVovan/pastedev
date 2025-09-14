import { API_URL } from '../constants';

export interface SnippetData {
  content: string;
  ephemeral: boolean;
  expiresAt?: Date;
}

export interface SnippetService {
  fetchSnippet: (id: string) => Promise<SnippetData>;
  saveSnippet: (content: string) => Promise<string>;
}

export const snippetService: SnippetService = {
  async fetchSnippet(id: string): Promise<SnippetData> {
    const response = await fetch(`${API_URL}/api/snippets/${id}`);

    if (!response.ok) {
      if (response.status === 404) {
        throw new Error('SNIPPET_NOT_FOUND');
      }
      throw new Error('Failed to fetch snippet');
    }

    const content = await response.text();
    const ephemeral = response.headers.get('X-Ephemeral') === 'true';
    const expiresAtHeader = response.headers.get('X-Expires-At');
    const expiresAt = expiresAtHeader ? new Date(expiresAtHeader) : undefined;

    return {
      content,
      ephemeral,
      expiresAt,
    };
  },

  async saveSnippet(content: string): Promise<string> {
    if (!content.trim()) {
      throw new Error('Content cannot be empty');
    }

    const response = await fetch(`${API_URL}/api/snippets`, {
      method: 'POST',
      headers: {
        'Content-Type': 'text/plain',
      },
      body: content,
    });

    if (!response.ok) {
      throw new Error('Failed to save snippet');
    }

    const url = await response.text();
    const id = url.split('/').pop();

    if (!id) {
      throw new Error('Invalid response from server');
    }

    return id;
  },
};
