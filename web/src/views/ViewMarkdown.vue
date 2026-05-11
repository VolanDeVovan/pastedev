<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import { renderMarkdown } from '../lib/markdown';
import { useAuthStore } from '../stores/auth';
import { HttpError } from '../api';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const snippet = ref<Snippet | null>(null);
const html = ref('');
const error = ref<string | null>(null);
const copied = ref(false);
const showDelete = ref(false);

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
  if (s?.type === 'markdown') html.value = renderMarkdown(s.body);
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
    <div class="max-w-[720px] mx-auto px-4 md:px-7 py-5 md:py-8">
      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="snippet">
        <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-3 mb-4">
          <div class="min-w-0">
            <div class="flex items-center gap-2 text-[11px] tracking-widest uppercase text-text-dim">
              <span>markdown</span>
              <span class="text-text-muted">·</span>
              <span class="text-accent truncate">{{ snippet.slug }}</span>
            </div>
            <h1 class="text-[16px] md:text-[18px] mt-1 tracking-tight truncate">{{ snippet.name ?? '(untitled)' }}</h1>
            <div class="text-[11px] text-text-muted mt-1.5">
              by {{ snippet.owner.username }} · {{ new Date(snippet.created_at).toLocaleString() }} · {{ snippet.views }} views
            </div>
          </div>
          <div class="flex gap-3 text-[12px] -mx-1 px-1 overflow-x-auto">
            <button class="text-text-muted hover:text-text whitespace-nowrap" @click="copyLink">{{ copied ? 'copied!' : 'copy link' }}</button>
            <a class="text-text-muted hover:text-text whitespace-nowrap" :href="snippet.raw_url" target="_blank">source ↗</a>
            <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline whitespace-nowrap">edit</RouterLink>
            <button v-if="canEdit(snippet)" class="text-danger hover:underline whitespace-nowrap" @click="showDelete = true">delete</button>
          </div>
        </div>
        <article class="md-preview" v-html="html" />
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
