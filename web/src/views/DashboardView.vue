<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import * as api from '../api';
import type { SnippetListItem } from '../api';
import type { SnippetType } from '../api/types';
import { Trash2 } from 'lucide-vue-next';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import { useToastStore } from '../stores/toast';
import { HttpError } from '../api';

const toast = useToastStore();

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

// Delete confirmation flow: tapping the row's delete button parks the item
// in `pendingDelete` and opens the shared Modal. The actual call only fires
// on confirm — accidental clicks land here, not in the API.
const pendingDelete = ref<SnippetListItem | null>(null);
const deleting = ref(false);

async function confirmDelete() {
  const item = pendingDelete.value;
  if (!item) return;
  deleting.value = true;
  try {
    await api.deleteSnippet(item.slug);
    // Remove locally rather than refetching — the listing is paginated and
    // a refetch would also wipe whatever pages the user has loaded.
    all.value = all.value.filter((i) => i.slug !== item.slug);
    toast.success(`deleted ${item.slug}`);
    pendingDelete.value = null;
  } catch (e) {
    toast.error(e instanceof HttpError ? e.error.message : 'delete failed');
  } finally {
    deleting.value = false;
  }
}

// Live "is this snippet already past its server-side expiry" check. Reactive
// to the periodic `now` tick below so a snippet that crosses the boundary
// while the page is open flips from "live" to "expired" without a reload.
// 30s cadence is plenty for "did this slug just cross its TTL" — the
// per-snippet viewer has its own 1s timer.
const now = ref(Date.now());
const nowHandle = window.setInterval(() => (now.value = Date.now()), 30_000);
onBeforeUnmount(() => clearInterval(nowHandle));

function isExpired(item: SnippetListItem): boolean {
  if (!item.expires_at) return false;
  return new Date(item.expires_at).getTime() <= now.value;
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
    <div class="px-4 md:px-7 py-5 md:py-7">
      <div class="flex flex-col md:flex-row md:items-end md:justify-between gap-3 mb-4 md:mb-5">
        <div>
          <h1 class="text-[20px] md:text-[22px] tracking-tight">my snippets</h1>
          <p class="text-[12px] text-text-muted mt-1 md:mt-1.5">
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
          <label class="flex items-center gap-2 bg-bg-deep border border-border rounded-sm px-3 py-2 text-[12px] text-text-muted flex-1 md:flex-none md:w-56 focus-within:border-accent transition-colors">
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
            class="bg-accent text-bg-deep font-semibold px-3 md:px-3.5 py-2 text-[12px] rounded-sm hover:opacity-90 shrink-0"
          >+ new<span class="hidden md:inline"> snippet</span></RouterLink>
        </div>
      </div>

      <div class="flex items-center gap-1.5 mb-1 text-[11px] -mx-1 px-1 overflow-x-auto">
        <button
          v-for="f in (['all', 'code', 'markdown', 'html'] as const)"
          :key="f"
          @click="filter = f"
          :class="[
            'px-3 md:px-3.5 py-1.5 md:py-2 rounded-sm border whitespace-nowrap shrink-0',
            filter === f
              ? 'bg-border text-text border-border-strong'
              : 'border-transparent text-text-muted hover:text-text',
          ]"
        >{{ f === 'markdown' ? 'md' : f }} · {{ counts[f] }}</button>
        <span v-if="query" class="ml-auto text-[11px] text-text-muted whitespace-nowrap">
          {{ items.length }} match{{ items.length === 1 ? '' : 'es' }}
          <button class="ml-2 text-accent hover:underline" @click="query = ''">clear</button>
        </span>
      </div>

      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>

      <!-- Desktop table header. On mobile the rows render as stacked cards
           (see mobile.jsx MList), so the header would be misleading. -->
      <div class="hidden md:grid grid-cols-[60px_1fr_140px_120px_90px_80px_40px] gap-4 py-2.5 border-b border-border text-[10px] tracking-widest uppercase text-text-muted">
        <div>type</div>
        <div>name</div>
        <div>url</div>
        <div>created</div>
        <div>size</div>
        <div class="text-right">views</div>
        <div></div>
      </div>

      <div v-if="loading && all.length === 0" class="text-[12px] text-text-muted py-4">loading…</div>
      <div v-else-if="!loading && items.length === 0" class="text-[12px] text-text-muted py-4">
        <template v-if="all.length === 0">
          no snippets yet — head to <RouterLink to="/" class="text-accent">new</RouterLink>.
        </template>
        <template v-else>nothing in this filter.</template>
      </div>

      <!-- One row per snippet. The outer element is a <div> (not a
           <RouterLink>) because we need to put a "delete" button inside it
           without it acting as navigation. The clickable area is the inner
           <RouterLink> on the row's main content; the trailing button column
           stays outside the link so its click event isn't swallowed. -->
      <div
        v-for="i in items"
        :key="i.slug"
        class="group flex md:grid md:grid-cols-[60px_1fr_140px_120px_90px_80px_40px] md:gap-4 gap-3 py-3 border-b border-border text-[13px] items-center hover:bg-bg-deep/40"
      >
        <RouterLink
          :to="pathFor(i)"
          class="contents"
        >
          <!-- Type chip (shared between layouts). -->
          <div class="shrink-0">
            <span
              :class="[
                typeColor(i.type),
                'inline-block px-2 py-0.5 rounded-sm text-[10px] border border-current/30 min-w-[2rem] text-center',
              ]"
            >{{ typeLabel(i.type) }}</span>
          </div>
          <!-- Mobile card body: name on top, slug + meta below. -->
          <div class="md:hidden flex-1 min-w-0">
            <div class="flex items-center gap-2 text-text truncate text-[13px]">
              <template v-if="i.name">{{ i.name }}</template>
              <span v-else class="text-text-muted">(untitled)</span>
              <span v-if="i.visibility === 'private'" class="text-warn text-[9px] uppercase tracking-widest">private</span>
              <span v-if="i.burn_after_read" class="text-amber-300 text-[9px] uppercase tracking-widest">burn</span>
              <span v-if="isExpired(i)" class="text-danger text-[9px] uppercase tracking-widest">expired</span>
            </div>
            <div class="flex gap-2 mt-1 text-[10px] text-text-muted min-w-0">
              <span class="text-accent font-mono truncate">{{ pathFor(i) }}</span>
              <span>· {{ ago(i.created_at) }}</span>
              <span>· {{ i.views }}v</span>
            </div>
          </div>
          <span class="md:hidden text-text-faint text-[14px] shrink-0">›</span>
          <!-- Desktop columns. -->
          <div class="hidden md:flex items-center gap-2 text-text truncate">
            <span class="truncate">
              <template v-if="i.name">{{ i.name }}</template>
              <span v-else class="text-text-muted">(untitled)</span>
            </span>
            <span v-if="i.visibility === 'private'" class="shrink-0 text-warn text-[9px] uppercase tracking-widest">private</span>
            <span v-if="i.burn_after_read" class="shrink-0 text-amber-300 text-[9px] uppercase tracking-widest">burn</span>
            <span v-if="isExpired(i)" class="shrink-0 text-danger text-[9px] uppercase tracking-widest">expired</span>
          </div>
          <div class="hidden md:block text-accent text-[12px] font-mono truncate">{{ pathFor(i) }}</div>
          <div class="hidden md:block text-text-dim text-[12px]">{{ ago(i.created_at) }}</div>
          <div class="hidden md:block text-text-dim text-[12px]">{{ formatSize(i.size_bytes) }}</div>
          <div class="hidden md:block text-text-dim text-[12px] text-right">{{ i.views }}</div>
        </RouterLink>
        <!-- Trailing action column. Lives outside the RouterLink so clicks
             on the trash icon don't navigate. Only visible on hover on
             desktop; always present (smaller) on mobile inside the card. -->
        <button
          type="button"
          class="ml-auto md:ml-0 text-text-faint hover:text-danger px-2 py-1 md:opacity-0 md:group-hover:opacity-100 transition-opacity shrink-0"
          :title="`delete ${i.slug}`"
          :aria-label="`delete ${i.slug}`"
          @click.stop.prevent="pendingDelete = i"
        ><Trash2 :size="14" /></button>
      </div>

      <button
        v-if="nextCursor"
        class="mt-4 text-[12px] text-text-muted hover:text-text"
        @click="loadMore"
      >load more →</button>
    </div>
    <Modal
      :open="pendingDelete !== null"
      title="delete snippet?"
      danger
      @update:open="(v) => { if (!v) pendingDelete = null; }"
      @confirm="confirmDelete"
    >
      <template v-if="pendingDelete">
        delete <code class="text-text">{{ pendingDelete.slug }}</code>?
        the slug stops resolving immediately.
      </template>
      <template #actions>
        <button
          type="button"
          class="text-text-muted hover:text-text px-3 py-1.5 text-[12px]"
          :disabled="deleting"
          @click="pendingDelete = null"
        >cancel</button>
        <button
          type="button"
          class="bg-danger/10 text-danger border border-danger-border rounded-sm px-3 py-1.5 text-[12px] hover:bg-danger/20 disabled:opacity-50"
          :disabled="deleting"
          @click="confirmDelete"
        >{{ deleting ? 'deleting…' : 'delete' }}</button>
      </template>
    </Modal>
  </Shell>
</template>
