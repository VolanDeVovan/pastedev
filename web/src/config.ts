// Runtime config injected by the server into index.html under <script id="paste-config">.
// Falls back to safe defaults so `vite dev` works without the Rust server in front.

export interface PasteConfig {
  apiBaseUrl: string;
  publicBaseUrl: string;
  appName: string;
}

const defaults: PasteConfig = {
  apiBaseUrl: '',
  publicBaseUrl: typeof location !== 'undefined' ? location.origin : '',
  appName: 'paste',
};

function read(): PasteConfig {
  if (typeof document === 'undefined') return defaults;
  const el = document.getElementById('paste-config');
  if (!el || !el.textContent) return defaults;
  try {
    const parsed = JSON.parse(el.textContent) as Partial<PasteConfig>;
    return { ...defaults, ...parsed };
  } catch {
    return defaults;
  }
}

export const config: PasteConfig = read();
