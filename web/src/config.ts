// Runtime config injected by the server into index.html under <script id="pastedev-config">.
// Falls back to safe defaults so `vite dev` works without the Rust server in front.

export interface PasteConfig {
  apiBaseUrl: string;
  publicBaseUrl: string;
  appName: string;
}

const defaults: PasteConfig = {
  apiBaseUrl: '',
  publicBaseUrl: typeof location !== 'undefined' ? location.origin : '',
  appName: 'pastedev',
};

function read(): PasteConfig {
  if (typeof document === 'undefined') return defaults;
  const el = document.getElementById('pastedev-config');
  if (!el || !el.textContent) return defaults;
  try {
    const parsed = JSON.parse(el.textContent) as Partial<PasteConfig>;
    return { ...defaults, ...parsed };
  } catch {
    return defaults;
  }
}

export const config: PasteConfig = read();
