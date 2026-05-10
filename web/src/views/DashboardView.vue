<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import * as api from '../api';
import type { SnippetListItem } from '../api';
import type { SnippetType } from '../api/types';
import Shell from '../components/Shell.vue';
import { HttpError } from '../api';

// We fetch the full list once (with paginated `load more` for tail) and filter
// client-side. This keeps the per-tab counts cheap and always in sync — the
// alternative of issuing one listSnippets() per type would still need a third
// "all" call to get the grand total, and the typical user has tens, not
// thousands, of snippets.
const all = ref<SnippetListItem[]>([]);
const filter = ref<SnippetType | 'all'>('all');
const query = ref('');
const loading = ref(false);
const error = ref<string | null>(null);
const nextCursor = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const list = await api.listSnippets({ limit: 50 });
    all.value = list.items;
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
      cursor: nextCursor.value,
      limit: 50,
    });
    all.value = all.value.concat(list.items);
    nextCursor.value = list.next_cursor;
  } finally {
    loading.value = false;
  }
}

// Filter by tab + free-text search. Search matches name OR slug (case-insensitive).
// The plan reserved search-inside-body as out-of-scope, so we only match metadata.
const items = computed(() => {
  const byType = filter.value === 'all'
    ? all.value
    : all.value.filter((i) => i.type === filter.value);
  const q = query.value.trim().toLowerCase();
  if (!q) return byType;
  return byType.filter((i) =>
    (i.name?.toLowerCase().includes(q) ?? false) || i.slug.toLowerCase().includes(q),
  );
});

const counts = computed(() => ({
  all: all.value.length,
  code: all.value.filter((i) => i.type === 'code').length,
  markdown: all.value.filter((i) => i.type === 'markdown').length,
  html: all.value.filter((i) => i.type === 'html').length,
}));

const totalBytes = computed(() => all.value.reduce((n, i) => n + i.size_bytes, 0));

const oldestAgo = computed(() => {
  if (all.value.length === 0) return null;
  const t = all.value.reduce(
    (min, i) => Math.min(min, new Date(i.created_at).getTime()),
    Date.now(),
  );
  return ago(new Date(t).toISOString());
});

function pathFor(item: SnippetListItem) {
  const prefix = item.type === 'code' ? '/c/' : item.type === 'markdown' ? '/m/' : '/h/';
  return `${prefix}${item.slug}`;
}

function typeLabel(t: SnippetType): string {
  return t === 'markdown' ? 'md' : t;
}

function typeColor(t: SnippetType): string {
  switch (t) {
    case 'code': return 'text-blue-300';
    case 'markdown': return 'text-emerald-300';
    case 'html': return 'text-amber-300';
  }
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} b`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} kb`;
  return `${(bytes / 1024 / 1024).toFixed(1)} mb`;
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
      <div class="flex items-end justify-between mb-5">
        <div>
          <h1 class="text-[22px] tracking-tight">my snippets</h1>
          <p class="text-[12px] text-text-muted mt-1.5">
            <template v-if="all.length === 0">no snippets yet</template>
            <template v-else>
              {{ all.length }} published · {{ formatSize(totalBytes) }} total
              <template v-if="oldestAgo"> · oldest from {{ oldestAgo }}</template>
            </template>
          </p>
        </div>
        <div class="flex gap-2">
          <!-- Search filters by name OR slug, client-side. plan/01-overview.html
               explicitly puts body-search out of scope, so this is a metadata
               filter, not full-text. -->
          <label class="flex items-center gap-2 bg-bg-deep border border-border rounded-sm px-3 py-2 text-[12px] text-text-muted w-56 focus-within:border-accent transition-colors">
            <span aria-hidden="true">⌕</span>
            <input
              v-model="query"
              type="search"
              placeholder="filter…"
              class="bg-transparent text-text focus:outline-none flex-1 min-w-0"
            />
          </label>
          <RouterLink
            to="/"
            class="bg-accent text-bg-deep font-semibold px-3.5 py-2 text-[12px] rounded-sm hover:opacity-90"
          >+ new snippet</RouterLink>
        </div>
      </div>

      <div class="flex items-center gap-1.5 mb-1 text-[11px]">
        <button
          v-for="f in (['all', 'code', 'markdown', 'html'] as const)"
          :key="f"
          @click="filter = f"
          :class="[
            'px-3.5 py-2 rounded-sm border',
            filter === f
              ? 'bg-border text-text border-border-strong'
              : 'border-transparent text-text-muted hover:text-text',
          ]"
        >{{ f }} · {{ counts[f] }}</button>
        <span v-if="query" class="ml-auto text-[11px] text-text-muted">
          {{ items.length }} match{{ items.length === 1 ? '' : 'es' }}
          <button class="ml-2 text-accent hover:underline" @click="query = ''">clear</button>
        </span>
      </div>

      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>

      <div class="grid grid-cols-[60px_1fr_140px_120px_90px_80px] gap-4 py-2.5 border-b border-border text-[10px] tracking-widest uppercase text-text-muted">
        <div>type</div>
        <div>name</div>
        <div>url</div>
        <div>created</div>
        <div>size</div>
        <div class="text-right">views</div>
      </div>

      <div v-if="loading && all.length === 0" class="text-[12px] text-text-muted py-4">loading…</div>
      <div v-else-if="!loading && items.length === 0" class="text-[12px] text-text-muted py-4">
        <template v-if="all.length === 0">
          no snippets yet — head to <RouterLink to="/" class="text-accent">new</RouterLink>.
        </template>
        <template v-else>nothing in this filter.</template>
      </div>

      <RouterLink
        v-for="i in items"
        :key="i.slug"
        :to="pathFor(i)"
        class="grid grid-cols-[60px_1fr_140px_120px_90px_80px] gap-4 py-3 border-b border-border text-[13px] items-center hover:bg-bg-deep/40"
      >
        <div>
          <span
            :class="[
              typeColor(i.type),
              'inline-block px-2 py-0.5 rounded-sm text-[10px] border border-current/30',
            ]"
          >{{ typeLabel(i.type) }}</span>
        </div>
        <div class="text-text truncate">
          <template v-if="i.name">{{ i.name }}</template>
          <span v-else class="text-text-muted">(untitled)</span>
        </div>
        <div class="text-accent text-[12px] font-mono truncate">{{ pathFor(i) }}</div>
        <div class="text-text-dim text-[12px]">{{ ago(i.created_at) }}</div>
        <div class="text-text-dim text-[12px]">{{ formatSize(i.size_bytes) }}</div>
        <div class="text-text-dim text-[12px] text-right">{{ i.views }}</div>
      </RouterLink>

      <button
        v-if="nextCursor"
        class="mt-4 text-[12px] text-text-muted hover:text-text"
        @click="loadMore"
      >load more →</button>
    </div>
  </Shell>
</template>
