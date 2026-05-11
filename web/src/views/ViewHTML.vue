<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import { useAuthStore } from '../stores/auth';
import { HttpError } from '../api';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const snippet = ref<Snippet | null>(null);
const error = ref<string | null>(null);
const copied = ref(false);
const showDelete = ref(false);

// The iframe grows to fit its content so the page scrolls naturally — no
// nested scrollbar inside the rendered html. Start with a sensible minimum
// while the bundled height-reporter script does its first measurement.
const iframeHeight = ref(400);
const iframeRef = ref<HTMLIFrameElement | null>(null);

onMounted(async () => {
  window.addEventListener('message', onHeightMessage);
  await load();
});

onBeforeUnmount(() => {
  window.removeEventListener('message', onHeightMessage);
});

function onHeightMessage(e: MessageEvent) {
  // Only trust messages from our own iframe — without this check any
  // window in the embedder chain could spoof the height.
  if (!iframeRef.value || e.source !== iframeRef.value.contentWindow) return;
  const data = e.data as { type?: string; height?: number } | null;
  if (data?.type === 'pastedev:height' && typeof data.height === 'number') {
    // Add a couple px of slack — Chromium occasionally rounds scrollHeight
    // down which leaves a 1-2px scrollbar inside the iframe.
    iframeHeight.value = Math.max(80, Math.ceil(data.height) + 2);
  }
}

async function load() {
  error.value = null;
  try {
    snippet.value = await api.getSnippet(route.params.slug as string);
    if (snippet.value.type !== 'html') {
      const prefix = snippet.value.type === 'markdown' ? '/m/' : '/c/';
      router.replace(`${prefix}${snippet.value.slug}`);
    }
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'load failed';
    snippet.value = null;
  }
}

// Wrap the user's html with a small height-reporter script. We render via
// `srcdoc` (instead of fetching from /h/:slug/raw via `src`) so we can splice
// the script in without depending on a server-side rewrite. The iframe stays
// sandboxed with allow-scripts only — no allow-same-origin — so user code
// still can't touch app cookies / storage. The reporter posts its
// scrollHeight on load and on any subsequent layout change.
const srcdoc = computed(() => {
  if (!snippet.value) return '';
  const userBody = snippet.value.body;
  const reporter = `
<script>
  (function () {
    function post() {
      try {
        var h = Math.max(
          document.documentElement.scrollHeight,
          document.body ? document.body.scrollHeight : 0
        );
        parent.postMessage({ type: 'pastedev:height', height: h }, '*');
      } catch (_) {}
    }
    if (document.readyState === 'complete') post();
    else window.addEventListener('load', post);
    if (typeof ResizeObserver === 'function') {
      new ResizeObserver(post).observe(document.documentElement);
    } else {
      setInterval(post, 500);
    }
  })();
<\/script>`;
  // Splice before </body> when present — keeps the user's <head> intact and
  // doesn't break documents that rely on body-end scripts. Falls back to
  // appending for fragments that omit the boilerplate.
  if (/<\/body\s*>/i.test(userBody)) {
    return userBody.replace(/<\/body\s*>/i, `${reporter}</body>`);
  }
  return `${userBody}${reporter}`;
});

async function copyLink() {
  if (!snippet.value) return;
  await navigator.clipboard.writeText(snippet.value.url);
  copied.value = true;
  setTimeout(() => (copied.value = false), 1500);
}
async function remove() {
  if (!snippet.value) return;
  showDelete.value = false;
  try {
    await api.deleteSnippet(snippet.value.slug);
    router.replace('/dashboard');
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'delete failed';
  }
}
const canEdit = (s: Snippet | null) => !!s && auth.user?.username === s.owner.username;
</script>

<template>
  <Shell>
    <div class="px-4 md:px-7 py-5 md:py-6">
      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="snippet && snippet.type === 'html'">
        <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-3 mb-3">
          <div class="min-w-0">
            <div class="flex items-center gap-2 text-[11px] tracking-widest uppercase text-warn">
              <span>html</span>
              <span class="text-text-muted">·</span>
              <span class="text-accent truncate">{{ snippet.slug }}</span>
            </div>
            <h1 class="text-[16px] md:text-[18px] mt-1 tracking-tight truncate">{{ snippet.name ?? '(untitled)' }}</h1>
            <div class="text-[11px] text-text-muted mt-1.5">
              by {{ snippet.owner.username }} · {{ new Date(snippet.created_at).toLocaleString() }} · {{ snippet.views }} views · {{ snippet.size_bytes }} b
            </div>
          </div>
          <div class="flex gap-3 text-[12px] -mx-1 px-1 overflow-x-auto">
            <button class="text-text-muted hover:text-text whitespace-nowrap" @click="copyLink">{{ copied ? 'copied!' : 'copy link' }}</button>
            <a class="text-text-muted hover:text-text whitespace-nowrap" :href="snippet.raw_url" target="_blank">open ↗</a>
            <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline whitespace-nowrap">edit</RouterLink>
            <button v-if="canEdit(snippet)" class="text-danger hover:underline whitespace-nowrap" @click="showDelete = true">delete</button>
          </div>
        </div>
        <div class="border border-warn/40 bg-warn/5 px-3 md:px-3.5 py-2 text-[9px] md:text-[10px] uppercase tracking-widest text-warn mb-2 rounded-sm">
          user-published html · sandboxed (no app-origin access)
        </div>
        <iframe
          ref="iframeRef"
          :srcdoc="srcdoc"
          sandbox="allow-scripts allow-popups"
          referrerpolicy="no-referrer"
          scrolling="no"
          :title="`${snippet.name ?? snippet.slug} · user html`"
          class="w-full block bg-white border border-border rounded-sm"
          :style="{ height: iframeHeight + 'px' }"
        />
      </div>
    </div>
    <Modal v-model:open="showDelete" title="delete snippet?" danger @confirm="remove">
      <template v-if="snippet">
        delete <code class="text-text">{{ snippet.slug }}</code>? this action cannot be undone.
        the slug stops resolving immediately.
      </template>
      <template #actions>
        <button
          type="button"
          class="text-text-muted hover:text-text px-3 py-1.5 text-[12px]"
          @click="showDelete = false"
        >cancel</button>
        <button
          type="button"
          class="bg-danger/10 text-danger border border-danger-border rounded-sm px-3 py-1.5 text-[12px] hover:bg-danger/20"
          @click="remove"
        >delete</button>
      </template>
    </Modal>
  </Shell>
</template>
