<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, toRef, watch } from 'vue';
import { useRouter } from 'vue-router';
import { ChevronUp, Clock, Flame, ShieldCheck } from 'lucide-vue-next';
import type { Snippet } from '../api';
import type { Visibility } from '../api/types';
import { LIFETIME_LABEL, type LifetimeKey } from '../lib/lifetime';
import { formatDuration, useSnippetCountdown } from '../composables/useSnippetCountdown';
import { useAuthStore } from '../stores/auth';

// Floating pill + command palette that replaces the header/toolbar chrome on
// the full-bleed html view. Collapsed it's a small pill in the corner (slug,
// countdown, burn flag); ⌘K / `.` / click expands it into a filterable command
// list with sub-views for visibility, expiry and delete confirmation.
//
// Single-key shortcuts (c / o / e / .) only fire while focus is in the app
// document; ⌘K / Ctrl+K also works from inside the iframe via the key bridge.

const props = defineProps<{
  snippet: Snippet;
  canEdit: boolean;
  pending?: boolean;
  /// The sandboxed iframe, so forwarded shortcuts can be origin-checked.
  frame?: HTMLIFrameElement | null;
}>();

const emit = defineEmits<{
  (e: 'commit', patch: { visibility?: Visibility; lifetimeKey?: LifetimeKey; burnAfterRead?: boolean }): void;
  (e: 'remove'): void;
}>();

const router = useRouter();
const auth = useAuthStore();
const { secondsLeft, expired } = useSnippetCountdown(toRef(props, 'snippet'));

type View = 'root' | 'visibility' | 'expires' | 'delete';

const open = ref(false);
const view = ref<View>('root');
const query = ref('');
const sel = ref(0);
const copied = ref(false);
const inputRef = ref<HTMLInputElement | null>(null);
const listRef = ref<HTMLDivElement | null>(null);

const TEAL = 'text-accent';
const RED = 'text-danger';
const AMBER = 'text-amber-300';

const expiryShort = computed(() => {
  if (expired.value) return 'expired';
  if (secondsLeft.value !== null) return formatDuration(secondsLeft.value);
  return null;
});

const meta = computed(() => {
  const s = props.snippet;
  const created = new Date(s.created_at).toLocaleString(undefined, {
    dateStyle: 'short',
    timeStyle: 'short',
  });
  return `by ${s.owner.username} · ${created} · ${s.views} views · ${s.size_bytes} b`;
});

interface Row {
  header?: string;
  label?: string;
  hint?: string;
  hintClass?: string;
  labelClass?: string;
  run?: () => void;
}

function close() {
  open.value = false;
  view.value = 'root';
  query.value = '';
  sel.value = 0;
  copied.value = false;
}

function go(v: View) {
  view.value = v;
  query.value = '';
  sel.value = 0;
  inputRef.value?.focus();
}

function navigate(to: string) {
  close();
  router.push(to);
}

async function copyLink() {
  await navigator.clipboard.writeText(props.snippet.url);
  copied.value = true;
}

function openRaw() {
  close();
  window.open(props.snippet.raw_url, '_blank', 'noopener');
}

function commit(patch: { visibility?: Visibility; lifetimeKey?: LifetimeKey; burnAfterRead?: boolean }) {
  if (props.pending) return;
  emit('commit', patch);
}

const rows = computed<Row[]>(() => {
  const s = props.snippet;
  if (view.value === 'visibility') {
    const pick = (v: Visibility) => () => {
      commit({ visibility: v });
      go('root');
    };
    const mark = (v: Visibility) => (s.visibility === v ? '✓' : '');
    return [
      { header: 'who can open the link' },
      { label: 'public — anyone with the link', hint: mark('public'), hintClass: TEAL, run: pick('public') },
      { label: 'private — signed-in users only', hint: mark('private'), hintClass: TEAL, run: pick('private') },
    ];
  }
  if (view.value === 'expires') {
    const pick = (k: LifetimeKey) => () => {
      commit({ lifetimeKey: k });
      go('root');
    };
    return [
      { header: 'stop resolving for non-owners' },
      ...(['15m', '1h', '1d', '1w'] as LifetimeKey[]).map((k) => ({
        label: `${LIFETIME_LABEL[k]} from now`,
        run: pick(k),
      })),
      { label: 'never expire', hint: s.expires_at ? '' : '✓', hintClass: TEAL, run: pick('never') },
    ];
  }
  if (view.value === 'delete') {
    return [
      { header: 'this cannot be undone' },
      { label: 'cancel', run: () => go('root') },
      {
        label: `delete ${s.slug} permanently`,
        labelClass: RED,
        run: () => {
          close();
          emit('remove');
        },
      },
    ];
  }

  const out: Row[] = [
    { header: 'snippet' },
    { label: 'copy link', hint: copied.value ? 'copied ✓' : 'c', hintClass: copied.value ? TEAL : '', run: copyLink },
    { label: 'open raw ↗', hint: 'o', run: openRaw },
  ];
  if (props.canEdit) {
    out.push({ label: 'edit', hint: 'e', run: () => navigate(`/?edit=${s.slug}`) });
    out.push(
      { header: 'access' },
      { label: 'visibility', hint: `${s.visibility} ›`, run: () => go('visibility') },
      {
        label: 'expires',
        hint: `${expiryShort.value === null ? 'never' : expired.value ? 'expired' : `in ${expiryShort.value}`} ›`,
        hintClass: expired.value ? RED : expiryShort.value ? TEAL : '',
        run: () => go('expires'),
      },
      {
        label: 'burn after first view',
        hint: s.burn_after_read ? 'on · 15m' : 'off',
        hintClass: s.burn_after_read ? AMBER : '',
        run: () => commit({ burnAfterRead: !s.burn_after_read }),
      },
    );
  }
  out.push({ header: 'go to' });
  if (auth.isApproved) {
    out.push(
      { label: 'new snippet', run: () => navigate('/') },
      { label: 'my snippets', run: () => navigate('/dashboard') },
      { label: 'api keys', run: () => navigate('/keys') },
    );
    if (auth.isAdmin) out.push({ label: 'admin', run: () => navigate('/admin') });
  } else if (!auth.user) {
    out.push(
      { label: 'sign in', run: () => navigate('/signin') },
      { label: 'register', run: () => navigate('/register') },
    );
  }
  if (props.canEdit) {
    out.push({ header: 'danger' }, { label: 'delete snippet…', hint: '›', labelClass: RED, run: () => go('delete') });
  }
  return out;
});

// Filtering drops section headers — a flat list of matches reads better
// than headers with one stray row under each.
const visibleRows = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return rows.value;
  return rows.value.filter((r) => !r.header && r.label!.toLowerCase().includes(q));
});

const items = computed(() => {
  let i = -1;
  return visibleRows.value.map((r) => (r.header ? { ...r, idx: -1 } : { ...r, idx: ++i }));
});
const selectableCount = computed(() => items.value.filter((r) => r.idx >= 0).length);
const selIdx = computed(() => Math.min(sel.value, Math.max(0, selectableCount.value - 1)));

function runAt(idx: number) {
  const row = items.value.find((r) => r.idx === idx);
  row?.run?.();
}

function scrollSelectedIntoView() {
  nextTick(() => {
    listRef.value?.querySelector('[data-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  });
}

function onInputKey(e: KeyboardEvent) {
  const n = selectableCount.value;
  if (e.key === 'ArrowDown') {
    e.preventDefault();
    if (n) sel.value = (selIdx.value + 1) % n;
    scrollSelectedIntoView();
  } else if (e.key === 'ArrowUp') {
    e.preventDefault();
    if (n) sel.value = (selIdx.value - 1 + n) % n;
    scrollSelectedIntoView();
  } else if (e.key === 'Enter') {
    e.preventDefault();
    runAt(selIdx.value);
  } else if (e.key === 'Escape') {
    e.preventDefault();
    if (view.value !== 'root') go('root');
    else close();
  } else if (e.key === 'Backspace' && !query.value && view.value !== 'root') {
    e.preventDefault();
    go('root');
  }
}

watch(query, () => (sel.value = 0));
watch(open, (isOpen) => {
  if (isOpen) nextTick(() => inputRef.value?.focus());
});

function isTyping(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.isContentEditable);
}

function onGlobalKey(e: KeyboardEvent) {
  // `code`, not `key`: on a Cyrillic layout Ctrl+K reports key 'л'.
  if ((e.metaKey || e.ctrlKey) && e.code === 'KeyK') {
    e.preventDefault();
    togglePalette();
    return;
  }
  if (open.value || isTyping(e.target) || e.metaKey || e.ctrlKey || e.altKey) return;
  if (e.key === '.') {
    e.preventDefault();
    open.value = true;
  } else if (e.key === 'c') {
    copyLink();
  } else if (e.key === 'o') {
    openRaw();
  } else if (e.key === 'e' && props.canEdit) {
    navigate(`/?edit=${props.snippet.slug}`);
  }
}

function togglePalette() {
  if (open.value) close();
  else open.value = true;
}

// While focus is inside the sandboxed iframe its keydowns never reach this
// document; the server-injected key bridge (HTML_KEY_BRIDGE in
// snippets/handlers.rs) forwards ⌘K / Ctrl+K as a postMessage instead. Only
// trust our own frame — any other window could spoof the message.
function onFrameMessage(e: MessageEvent) {
  if (!props.frame || e.source !== props.frame.contentWindow) return;
  if ((e.data as { type?: string } | null)?.type !== 'pastedev:palette') return;
  // Pull focus out of the iframe so the palette input can take it.
  window.focus();
  togglePalette();
}

onMounted(() => {
  document.addEventListener('keydown', onGlobalKey);
  window.addEventListener('message', onFrameMessage);
});
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onGlobalKey);
  window.removeEventListener('message', onFrameMessage);
});

const isMac = typeof navigator !== 'undefined' && /Mac|iPhone|iPad/.test(navigator.platform);
</script>

<template>
  <!-- collapsed pill -->
  <button
    v-if="!open"
    type="button"
    aria-label="open pastedev commands"
    title="pastedev commands (press .)"
    class="fixed right-3.5 bottom-7 md:right-5 md:bottom-5 z-40 h-11 md:h-9 pl-3.5 pr-3.5 md:pr-3 flex items-center gap-2.5 rounded-full bg-bg/90 border border-border-strong shadow-[0_6px_24px_rgba(0,0,0,0.3)] text-[12px] text-text whitespace-nowrap backdrop-blur-sm hover:border-accent/50 transition-colors"
    @click="open = true"
  >
    <span class="font-bold tracking-tight">pd</span>
    <span class="hidden md:inline text-accent">{{ snippet.slug }}</span>
    <span
      v-if="expiryShort"
      class="inline-flex items-center gap-1 tabular-nums"
      :class="expired ? 'text-danger' : 'text-accent'"
    >
      <Clock :size="12" />{{ expiryShort }}
    </span>
    <Flame v-if="snippet.burn_after_read" :size="12" class="text-amber-300" />
    <span class="hidden md:inline text-[10px] text-text-muted border border-border-strong rounded-[3px] px-1.5 py-px">
      {{ isMac ? '⌘K' : 'ctrl K' }}
    </span>
    <ChevronUp :size="12" class="md:hidden text-text-muted" />
  </button>

  <template v-if="open">
    <button
      type="button"
      aria-label="close commands"
      tabindex="-1"
      class="fixed inset-[3px] z-40 bg-bg-deep/45 cursor-default"
      @click="close"
    />

    <div
      role="dialog"
      aria-label="pastedev commands"
      class="fixed z-50 left-3 right-3 top-[8vh] md:left-1/2 md:right-auto md:-translate-x-1/2 md:top-[120px] md:w-[560px] bg-bg border border-border-strong rounded-[10px] shadow-[0_24px_64px_rgba(0,0,0,0.45)] text-text text-[12px] flex flex-col overflow-hidden"
    >
      <div class="px-4 pt-3.5 pb-2.5 flex flex-col gap-[3px] border-b border-border">
        <div class="flex items-center gap-2 text-[11px] min-w-0">
          <span class="font-bold tracking-tight">pastedev</span>
          <span class="text-border-strong">/</span>
          <span class="text-warn uppercase tracking-widest text-[10px]">html</span>
          <span class="text-accent truncate">{{ snippet.slug }}</span>
          <span
            class="ml-auto shrink-0 inline-flex items-center gap-1 text-warn text-[10px] uppercase tracking-wider"
            title="user-published html · no app-origin access"
          >
            <ShieldCheck :size="11" />sandboxed
          </span>
        </div>
        <div class="text-[16px] break-words">{{ snippet.name ?? '(untitled)' }}</div>
        <div class="text-[11px] text-text-muted">{{ meta }}</div>
        <div v-if="expired" class="text-[11px] text-danger">
          expired — anyone else clicking the link now gets a 404.
        </div>
      </div>

      <div class="flex items-center gap-2.5 px-4 h-11 border-b border-border">
        <button
          v-if="view !== 'root'"
          type="button"
          aria-label="back"
          class="inline-flex items-center gap-1 px-2 py-0.5 rounded-[3px] bg-border text-text text-[11px] whitespace-nowrap"
          @click="go('root')"
        >‹ {{ view }}</button>
        <span v-else class="text-accent">›</span>
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          aria-label="command"
          :placeholder="view === 'root' ? 'type a command…' : 'filter…'"
          class="flex-1 min-w-0 bg-transparent border-0 outline-none text-text text-[13px] placeholder:text-text-faint"
          @keydown="onInputKey"
        />
        <span class="text-[10px] text-text-muted border border-border-strong rounded-[3px] px-1.5 py-px">esc</span>
      </div>

      <div ref="listRef" class="py-1.5 flex flex-col max-h-[50vh] md:max-h-[460px] overflow-auto">
        <template v-for="(it, i) in items" :key="i">
          <div
            v-if="it.header"
            class="px-4 pt-2.5 pb-1 text-[10px] tracking-widest uppercase text-text-muted"
          >{{ it.header }}</div>
          <button
            v-else
            type="button"
            :data-selected="it.idx === selIdx"
            class="flex items-center gap-2.5 px-4 h-[34px] w-full shrink-0 text-left"
            :class="[it.idx === selIdx ? 'bg-border' : 'bg-transparent', it.labelClass ?? 'text-text']"
            @mouseenter="sel = it.idx"
            @click="it.run?.()"
          >
            <span class="flex-1 truncate">{{ it.label }}</span>
            <span class="text-[11px]" :class="it.hintClass || 'text-text-muted'">{{ it.hint }}</span>
          </button>
        </template>
        <div v-if="selectableCount === 0" class="p-4 text-text-muted">no matching commands</div>
      </div>

      <div class="border-t border-border px-4 py-2 text-[10px] text-text-muted flex gap-4">
        <span>↑↓ move</span>
        <span>↵ run</span>
        <span>{{ view === 'root' ? 'esc close' : 'esc back' }}</span>
        <span v-if="pending" class="text-accent">saving…</span>
        <span v-if="auth.user" class="ml-auto truncate">{{ auth.user.username }}</span>
      </div>
    </div>

    <!-- pill stays put in its "active" state while the palette is open -->
    <div
      class="fixed right-3.5 bottom-7 md:right-5 md:bottom-5 z-40 h-11 md:h-9 px-3.5 flex items-center gap-2.5 rounded-full bg-border border border-accent/50 text-[12px] text-text whitespace-nowrap"
    >
      <span class="font-bold tracking-tight">pd</span>
      <span class="text-accent">{{ snippet.slug }}</span>
    </div>
  </template>
</template>
