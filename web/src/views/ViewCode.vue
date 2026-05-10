<script setup lang="ts">
import { onMounted, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
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

const { html: highlightedHtml, language, truncated: hlTruncated, highlight } = useHighlight();

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
  if (!confirm(`delete ${snippet.value.slug}?`)) return;
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
    <div class="max-w-5xl mx-auto px-7 py-7">
      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="snippet">
        <div class="flex items-end justify-between mb-4">
          <div>
            <div class="flex items-center gap-2 text-[11px] tracking-widest uppercase text-text-dim">
              <span>code</span>
              <span class="text-text-muted">·</span>
              <span class="text-accent">{{ snippet.slug }}</span>
            </div>
            <h1 class="text-[18px] mt-1 tracking-tight">{{ snippet.name ?? '(untitled)' }}</h1>
            <div class="text-[11px] text-text-muted mt-1.5">
              by {{ snippet.owner.username }} · {{ new Date(snippet.created_at).toLocaleString() }} · {{ snippet.views }} views · {{ snippet.size_bytes }} b
              <span v-if="language"> · {{ language }}</span>
            </div>
          </div>
          <div class="flex gap-3 text-[12px]">
            <button class="text-text-muted hover:text-text" @click="copyRaw">{{ copiedRaw ? 'copied!' : 'copy raw' }}</button>
            <button class="text-text-muted hover:text-text" @click="copyLink">{{ copiedLink ? 'copied!' : 'copy link' }}</button>
            <a class="text-text-muted hover:text-text" :href="snippet.raw_url" target="_blank">raw ↗</a>
            <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline">edit</RouterLink>
            <button v-if="canEdit(snippet)" class="text-danger hover:underline" @click="remove">delete</button>
          </div>
        </div>
        <pre class="bg-bg-deep border border-border rounded-sm px-4 py-4 text-[13px] leading-relaxed whitespace-pre-wrap break-words"><code class="hljs" v-html="highlightedHtml || snippet.body" /></pre>
        <div v-if="hlTruncated" class="text-[11px] text-warn mt-2">syntax highlighting off · large file ({{ snippet.size_bytes.toLocaleString() }} b)</div>
      </div>
    </div>
  </Shell>
</template>
