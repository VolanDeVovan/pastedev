<script setup lang="ts">
import { onMounted, ref } from 'vue';
import * as api from '../api';
import type { SnippetListItem } from '../api';
import type { SnippetType } from '../api/types';
import Shell from '../components/Shell.vue';
import { HttpError } from '../api';

const items = ref<SnippetListItem[]>([]);
const filter = ref<SnippetType | 'all'>('all');
const loading = ref(false);
const error = ref<string | null>(null);
const nextCursor = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const list = await api.listSnippets({
      type: filter.value === 'all' ? undefined : filter.value,
      limit: 50,
    });
    items.value = list.items;
    nextCursor.value = list.next_cursor;
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'failed to load';
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

async function loadMore() {
  if (!nextCursor.value) return;
  loading.value = true;
  try {
    const list = await api.listSnippets({
      type: filter.value === 'all' ? undefined : filter.value,
      cursor: nextCursor.value,
      limit: 50,
    });
    items.value = items.value.concat(list.items);
    nextCursor.value = list.next_cursor;
  } finally {
    loading.value = false;
  }
}

function pathFor(item: SnippetListItem) {
  const prefix = item.type === 'code' ? '/c/' : item.type === 'markdown' ? '/m/' : '/h/';
  return `${prefix}${item.slug}`;
}

function tag(t: SnippetType): string {
  switch (t) {
    case 'code': return 'text-blue-300';
    case 'markdown': return 'text-emerald-300';
    case 'html': return 'text-amber-300';
  }
}

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
    <div class="max-w-4xl mx-auto px-6 py-10">
      <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · dashboard</div>
      <h1 class="text-lg font-medium mb-6">Your snippets.</h1>

      <div class="flex items-center gap-4 mb-3 text-sm border-b border-border-strong">
        <button
          v-for="f in (['all', 'code', 'markdown', 'html'] as const)"
          :key="f"
          @click="filter = f; refresh()"
          :class="[
            'px-1 py-2 -mb-px border-b-2 text-xs uppercase tracking-widest',
            filter === f ? 'border-accent text-text' : 'border-transparent text-text-muted',
          ]"
        >{{ f }}</button>
        <RouterLink to="/" class="ml-auto text-xs text-accent hover:underline">+ new snippet</RouterLink>
      </div>

      <div v-if="error" class="text-sm text-rose-400 mb-4">{{ error }}</div>
      <div v-if="loading && items.length === 0" class="text-sm text-text-muted">loading…</div>
      <div v-if="!loading && items.length === 0" class="text-sm text-text-muted">No snippets yet.</div>

      <ul class="divide-y divide-border-strong">
        <li v-for="i in items" :key="i.slug" class="py-3 grid grid-cols-[60px_1fr_auto_auto_auto] gap-4 items-center text-sm">
          <span :class="[tag(i.type), 'uppercase text-[10px] tracking-widest']">{{ i.type }}</span>
          <RouterLink :to="pathFor(i)" class="text-text hover:text-accent truncate">
            <span class="text-text-muted">{{ i.slug }}</span>
            <span v-if="i.name" class="ml-3">{{ i.name }}</span>
          </RouterLink>
          <span class="text-xs text-text-muted">{{ ago(i.created_at) }}</span>
          <span class="text-xs text-text-muted">{{ i.size_bytes }} b</span>
          <span class="text-xs text-text-muted">{{ i.views }} views</span>
        </li>
      </ul>
      <button v-if="nextCursor" class="mt-4 text-xs text-text-muted hover:text-text" @click="loadMore">load more →</button>
    </div>
  </Shell>
</template>
