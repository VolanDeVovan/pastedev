<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
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
  if (!confirm(`delete ${snippet.value.slug}?`)) return;
  try {
    await api.deleteSnippet(snippet.value.slug);
    router.replace('/dashboard');
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'delete failed';
  }
}

const canEdit = (s: Snippet | null) =>
  !!s && auth.user?.username === s.owner.username;
</script>

<template>
  <Shell>
    <div class="max-w-3xl mx-auto px-6 py-8">
      <div v-if="error" class="text-sm text-rose-400 mb-4">{{ error }}</div>
      <div v-if="snippet">
        <div class="flex items-center justify-between mb-3">
          <div>
            <div class="text-[11px] uppercase tracking-widest text-accent">markdown · {{ snippet.slug }}</div>
            <h1 class="text-base font-medium mt-1">{{ snippet.name ?? '(untitled)' }}</h1>
          </div>
          <div class="flex gap-3 text-xs">
            <button class="text-text-muted hover:text-text" @click="copyLink">{{ copied ? 'copied!' : 'copy link' }}</button>
            <a class="text-text-muted hover:text-text" :href="snippet.raw_url" target="_blank">source ↗</a>
            <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline">edit</RouterLink>
            <button v-if="canEdit(snippet)" class="text-rose-400 hover:underline" @click="remove">delete</button>
          </div>
        </div>
        <div class="text-xs text-text-muted mb-4">
          by {{ snippet.owner.username }} · {{ new Date(snippet.created_at).toLocaleString() }} · {{ snippet.views }} views
        </div>
        <article class="prose prose-invert prose-sm max-w-none font-sans" v-html="html" />
      </div>
    </div>
  </Shell>
</template>
