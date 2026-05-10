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
    case 'code': return 'text-text-dim';
    case 'markdown': return 'text-emerald-300';
    case 'html': return 'text-warn';
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
    <div class="max-w-5xl mx-auto px-7 py-8">
      <h1 class="text-[22px] tracking-tight mb-1.5">my snippets</h1>
      <p class="text-[12px] text-text-muted mb-6">
        {{ items.length }} snippet{{ items.length === 1 ? '' : 's' }}
        <template v-if="filter !== 'all'"> · filtered to {{ filter }}</template>
      </p>

      <div class="flex items-center gap-4 mb-3 text-[12px] border-b border-border">
        <button
          v-for="f in (['all', 'code', 'markdown', 'html'] as const)"
          :key="f"
          @click="filter = f; refresh()"
          :class="[
            'px-1 pb-2 -mb-px border-b',
            filter === f ? 'border-accent text-text' : 'border-transparent text-text-muted hover:text-text',
          ]"
        >{{ f }}</button>
        <RouterLink to="/" class="ml-auto pb-2 text-accent hover:underline">+ new snippet</RouterLink>
      </div>

      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="loading && items.length === 0" class="text-[12px] text-text-muted py-4">loading…</div>
      <div v-if="!loading && items.length === 0" class="text-[12px] text-text-muted py-4">no snippets yet — head to <RouterLink to="/" class="text-accent">new</RouterLink>.</div>

      <ul class="divide-y divide-border">
        <li v-for="i in items" :key="i.slug" class="py-3 grid grid-cols-[44px_1fr_auto_auto_auto] gap-4 items-center text-[13px]">
          <span :class="[tag(i.type), 'uppercase text-[10px] tracking-widest']">{{ i.type === 'markdown' ? 'md' : i.type }}</span>
          <RouterLink :to="pathFor(i)" class="text-text hover:text-accent truncate">
            <span class="text-text-muted">{{ i.slug }}</span>
            <span v-if="i.name" class="ml-3">{{ i.name }}</span>
            <span v-else class="ml-3 text-text-muted">(untitled)</span>
          </RouterLink>
          <span class="text-[11px] text-text-muted">{{ ago(i.created_at) }}</span>
          <span class="text-[11px] text-text-muted">{{ i.size_bytes }} b</span>
          <span class="text-[11px] text-text-muted">{{ i.views }} views</span>
        </li>
      </ul>
      <button v-if="nextCursor" class="mt-4 text-[12px] text-text-muted hover:text-text" @click="loadMore">load more →</button>
    </div>
  </Shell>
</template>
