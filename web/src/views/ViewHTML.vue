<script setup lang="ts">
import { onMounted, ref, computed } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
import { useAuthStore } from '../stores/auth';
import { HttpError } from '../api';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const snippet = ref<Snippet | null>(null);
const error = ref<string | null>(null);
const copied = ref(false);

const rawPath = computed(() => (snippet.value ? `/h/${snippet.value.slug}/raw` : ''));

onMounted(load);

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
const canEdit = (s: Snippet | null) => !!s && auth.user?.username === s.owner.username;
</script>

<template>
  <Shell>
    <div class="max-w-6xl mx-auto px-7 py-6">
      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="snippet && snippet.type === 'html'">
        <div class="flex items-end justify-between mb-3">
          <div>
            <div class="flex items-center gap-2 text-[11px] tracking-widest uppercase text-warn">
              <span>html</span>
              <span class="text-text-muted">·</span>
              <span class="text-accent">{{ snippet.slug }}</span>
            </div>
            <h1 class="text-[18px] mt-1 tracking-tight">{{ snippet.name ?? '(untitled)' }}</h1>
            <div class="text-[11px] text-text-muted mt-1.5">
              by {{ snippet.owner.username }} · {{ new Date(snippet.created_at).toLocaleString() }} · {{ snippet.views }} views · {{ snippet.size_bytes }} b
            </div>
          </div>
          <div class="flex gap-3 text-[12px]">
            <button class="text-text-muted hover:text-text" @click="copyLink">{{ copied ? 'copied!' : 'copy link' }}</button>
            <a class="text-text-muted hover:text-text" :href="snippet.raw_url" target="_blank">open ↗</a>
            <RouterLink v-if="canEdit(snippet)" :to="`/?edit=${snippet.slug}`" class="text-accent hover:underline">edit</RouterLink>
            <button v-if="canEdit(snippet)" class="text-danger hover:underline" @click="remove">delete</button>
          </div>
        </div>
        <div class="border border-warn/40 bg-warn/5 px-3.5 py-2 text-[10px] uppercase tracking-widest text-warn mb-2 rounded-sm">
          user-published html · sandboxed (no app-origin access)
        </div>
        <iframe
          :src="rawPath"
          sandbox="allow-scripts allow-popups"
          referrerpolicy="no-referrer"
          :title="`${snippet.name ?? snippet.slug} · user html`"
          class="w-full h-[78vh] bg-white border border-border rounded-sm"
        />
      </div>
    </div>
  </Shell>
</template>
