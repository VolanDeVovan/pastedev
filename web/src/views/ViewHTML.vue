<script setup lang="ts">
import { onMounted, onBeforeUnmount, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import SnippetStatus from '../components/SnippetStatus.vue';
import PolicyBar from '../components/PolicyBar.vue';
import { LIFETIME_SECONDS, type LifetimeKey } from '../lib/lifetime';
import { useSnippetCountdown } from '../composables/useSnippetCountdown';
import { useAuthStore } from '../stores/auth';
import { useToastStore } from '../stores/toast';
import { HttpError } from '../api';
import type { Visibility } from '../api/types';
import { watch } from 'vue';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const snippet = ref<Snippet | null>(null);
const error = ref<string | null>(null);
const copied = ref(false);
const showDelete = ref(false);
const toast = useToastStore();
const savingSettings = ref(false);

const visibility = ref<Visibility>('public');
const burnAfterRead = ref(false);

watch(snippet, (s) => {
  if (!s) return;
  visibility.value = s.visibility;
  burnAfterRead.value = s.burn_after_read;
});

async function commitPolicy(patch: {
  visibility?: Visibility;
  lifetimeKey?: LifetimeKey;
  burnAfterRead?: boolean;
}) {
  if (!snippet.value) return;
  savingSettings.value = true;
  try {
    const apiPatch: {
      visibility?: Visibility;
      lifetime_seconds?: number | null;
      burn_after_read?: boolean;
    } = {};
    if (patch.visibility !== undefined) apiPatch.visibility = patch.visibility;
    if (patch.lifetimeKey !== undefined) apiPatch.lifetime_seconds = LIFETIME_SECONDS[patch.lifetimeKey];
    if (patch.burnAfterRead !== undefined) apiPatch.burn_after_read = patch.burnAfterRead;
    const updated = await api.updateSnippetSettings(snippet.value.slug, apiPatch);
    snippet.value = updated;
    toast.success('settings updated');
  } catch (e) {
    toast.error(e instanceof HttpError ? e.error.message : 'update failed');
  } finally {
    savingSettings.value = false;
  }
}

// The iframe grows to fit its content in both axes so the surrounding page
// (and a horizontally-scrollable wrapper) handle scrolling — no nested
// scrollbars inside the rendered html. Start with a sensible minimum while
// the server-injected size-reporter does its first measurement.
//
// We load via `src=/h/:slug/raw` rather than splicing srcdoc on the client:
// the SPA's own CSP is `script-src 'self'`, and srcdoc iframes inherit it,
// which silently blocks all inline scripts inside the preview — both the
// reporter and any scripts in the user's html. Loading a real document lets
// the response's own CSP (`sandbox allow-scripts allow-popups`) take effect.
// The iframe `sandbox` attribute (no allow-same-origin) still gives the
// preview a null origin, so user code cannot read app cookies / storage.
const iframeHeight = ref(400);
const iframeWidth = ref<number | null>(null);
const iframeRef = ref<HTMLIFrameElement | null>(null);

onMounted(async () => {
  window.addEventListener('message', onSizeMessage);
  await load();
});

onBeforeUnmount(() => {
  window.removeEventListener('message', onSizeMessage);
});

function onSizeMessage(e: MessageEvent) {
  // Only trust messages from our own iframe — without this check any
  // window in the embedder chain could spoof the dimensions.
  if (!iframeRef.value || e.source !== iframeRef.value.contentWindow) return;
  const data = e.data as { type?: string; height?: number; width?: number } | null;
  if (data?.type !== 'pastedev:size') return;
  if (typeof data.height === 'number') {
    commitHeight(Math.ceil(data.height));
  }
  if (typeof data.width === 'number') {
    commitWidth(Math.ceil(data.width));
  }
}

// `SIZE_SLACK` adds room for Chromium's occasional 1-2px round-down of
// scrollWidth/Height; without it the iframe ends up with a tiny internal
// scrollbar. `HYSTERESIS` skips updates within ±2*slack of the current value
// — that's exactly the bounce-back range when our last write echoed back
// through the iframe's ResizeObserver, so it breaks the feedback loop that
// would otherwise grow the iframe a few px on every animation frame.
const SIZE_SLACK = 2;
const SIZE_HYSTERESIS = SIZE_SLACK * 2;

function commitHeight(measured: number) {
  const target = Math.max(80, measured + SIZE_SLACK);
  if (Math.abs(target - iframeHeight.value) <= SIZE_HYSTERESIS) return;
  iframeHeight.value = target;
}

function commitWidth(measured: number) {
  const target = Math.max(0, measured + SIZE_SLACK);
  if (iframeWidth.value !== null && Math.abs(target - iframeWidth.value) <= SIZE_HYSTERESIS) return;
  iframeWidth.value = target;
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
    if (e instanceof HttpError && e.status === 401) {
      router.replace({ name: 'signin', query: { next: route.fullPath } });
      return;
    }
    error.value = e instanceof HttpError ? e.error.message : 'load failed';
    snippet.value = null;
  }
}

const { expired } = useSnippetCountdown(snippet);

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
            <h1 class="text-[16px] md:text-[18px] mt-1 tracking-tight break-words">{{ snippet.name ?? '(untitled)' }}</h1>
            <div class="text-[11px] text-text-muted mt-1.5">
              by {{ snippet.owner.username }} · {{ new Date(snippet.created_at).toLocaleString() }} · {{ snippet.views }} views · {{ snippet.size_bytes }} b
            </div>
          </div>
          <div class="flex flex-col md:flex-row md:items-center gap-3 md:gap-4">
            <PolicyBar
              v-if="canEdit(snippet)"
              v-model:visibility="visibility"
              v-model:burn-after-read="burnAfterRead"
              mode="remote"
              :pending="savingSettings"
              :expires-at="snippet.expires_at ?? null"
              @commit="commitPolicy"
            />
            <PolicyBar
              v-else
              v-model:visibility="visibility"
              v-model:burn-after-read="burnAfterRead"
              mode="inline"
              disabled
              :expires-at="snippet.expires_at ?? null"
            />
            <div class="flex gap-3 text-[12px] overflow-x-auto">
              <button class="text-text-muted hover:text-text whitespace-nowrap" @click="copyLink">{{ copied ? 'copied!' : 'copy link' }}</button>
              <a class="text-text-muted hover:text-text whitespace-nowrap" :href="snippet.raw_url" target="_blank">open ↗</a>
              <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline whitespace-nowrap">edit</RouterLink>
              <button v-if="canEdit(snippet)" class="text-danger hover:underline whitespace-nowrap" @click="showDelete = true">delete</button>
            </div>
          </div>
        </div>
        <SnippetStatus :snippet="snippet" />
        <div
          v-if="expired"
          class="mb-2 text-[11px] text-danger px-2 py-1.5 border border-danger-border rounded-sm bg-danger/5"
        >
          this snippet has expired — anyone else clicking the link now gets a 404.
        </div>
        <div class="border border-warn/40 bg-warn/5 px-3 md:px-3.5 py-2 text-[9px] md:text-[10px] uppercase tracking-widest text-warn mb-2 rounded-sm">
          user-published html · sandboxed (no app-origin access)
        </div>
        <!-- Wrap the iframe in a horizontally-scrollable container. The
             size-reporter (see crates/server/src/snippets/handlers.rs:
             HTML_SIZE_REPORTER) tells us the content's scrollWidth; we then
             size the iframe to that width and let this wrapper scroll. With
             `min-width: 100%` on the iframe it still stretches to fill on
             narrow content. -->
        <div class="overflow-x-auto">
          <iframe
            ref="iframeRef"
            :src="snippet.raw_url"
            sandbox="allow-scripts allow-popups"
            referrerpolicy="no-referrer"
            scrolling="no"
            :title="`${snippet.name ?? snippet.slug} · user html`"
            class="block bg-white border border-border rounded-sm"
            :style="{
              height: iframeHeight + 'px',
              width: iframeWidth ? iframeWidth + 'px' : '100%',
              minWidth: '100%',
            }"
          />
        </div>
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
