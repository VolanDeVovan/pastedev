<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import { useHighlight } from '../composables/useHighlight';
import { useAuthStore } from '../stores/auth';
import { HttpError } from '../api';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const snippet = ref<Snippet | null>(null);
const error = ref<string | null>(null);
const copiedLink = ref(false);
const copiedRaw = ref(false);
const showDelete = ref(false);

const { html: highlightedHtml, language, truncated: hlTruncated, highlight } = useHighlight();

// Line numbers double as the left "padding" — design mockups put a faint
// gutter to the left of the code instead of indenting the body with px-*.
// One number per logical newline; long wrapped lines still count as one.
const lineCount = computed(() => {
  if (!snippet.value) return 0;
  const body = snippet.value.body;
  if (body.length === 0) return 1;
  // `split('\n').length` counts a trailing empty line, which is desirable —
  // a file ending in `\n` should still show its last line numbered.
  return body.split('\n').length;
});

onMounted(load);

async function load() {
  error.value = null;
  try {
    snippet.value = await api.getSnippet(route.params.slug as string);
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'load failed';
    snippet.value = null;
  }
}

watch(snippet, (s) => {
  if (s?.type === 'code') highlight(s.body);
});

async function copyLink() {
  if (!snippet.value) return;
  await navigator.clipboard.writeText(snippet.value.url);
  copiedLink.value = true;
  setTimeout(() => (copiedLink.value = false), 1500);
}
async function copyRaw() {
  if (!snippet.value) return;
  await navigator.clipboard.writeText(snippet.value.body);
  copiedRaw.value = true;
  setTimeout(() => (copiedRaw.value = false), 1500);
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

function ago(iso: string): string {
  const d = new Date(iso);
  const s = Math.floor((Date.now() - d.getTime()) / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}
</script>

<template>
  <Shell>
    <div>
      <!-- Header strip — `padding: 20px 28px 12px` per screens.jsx ViewCode,
           with a divider underneath. Stacks on mobile so the action row
           doesn't elbow the title off-screen. -->
      <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-3 px-4 md:px-7 pt-5 md:pt-5 pb-3 border-b border-border">
        <div class="min-w-0">
          <div v-if="snippet" class="text-[16px] md:text-[18px] tracking-tight truncate">
            {{ snippet.name ?? '(untitled)' }}
          </div>
          <div v-if="snippet" class="text-text-muted text-[11px] mt-1 flex flex-wrap gap-x-3 gap-y-0.5">
            <span>{{ language ?? 'code' }}</span>
            <span>· {{ snippet.size_bytes }} b</span>
            <span>· by {{ snippet.owner.username }}</span>
            <span>· {{ ago(snippet.created_at) }}</span>
            <span class="text-accent truncate">· {{ snippet.slug }}</span>
          </div>
        </div>
        <div v-if="snippet" class="flex gap-3 text-[12px] -mx-1 px-1 overflow-x-auto md:overflow-visible">
          <button class="text-text-muted hover:text-text whitespace-nowrap" @click="copyRaw">{{ copiedRaw ? 'copied!' : 'copy raw' }}</button>
          <button class="text-text-muted hover:text-text whitespace-nowrap" @click="copyLink">{{ copiedLink ? 'copied!' : 'copy link' }}</button>
          <a class="text-text-muted hover:text-text whitespace-nowrap" :href="snippet.raw_url" target="_blank">raw ↗</a>
          <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline whitespace-nowrap">edit</RouterLink>
          <button v-if="canEdit(snippet)" class="text-danger hover:underline whitespace-nowrap" @click="showDelete = true">delete</button>
        </div>
      </div>

      <div v-if="error" class="text-[12px] text-danger px-4 md:px-7 py-4">{{ error }}</div>

      <!-- Code body — `padding: 20px 28px` and a flex `[gutter] [pre]` per
           the design. Line numbers are user-select:none so a "select all"
           on the page only grabs the code itself. -->
      <div v-if="snippet" class="flex font-mono text-[12px] md:text-[13px] leading-relaxed px-4 md:px-7 py-5">
        <div class="text-text-faint pr-3 md:pr-[18px] text-right select-none shrink-0">
          <div v-for="i in lineCount" :key="i">{{ i }}</div>
        </div>
        <pre class="m-0 whitespace-pre-wrap break-words flex-1 min-w-0"><code class="hljs" v-html="highlightedHtml || snippet.body" /></pre>
      </div>
      <div v-if="snippet && hlTruncated" class="text-[11px] text-warn px-4 md:px-7 pb-4">
        syntax highlighting off · large file ({{ snippet.size_bytes.toLocaleString() }} b)
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
