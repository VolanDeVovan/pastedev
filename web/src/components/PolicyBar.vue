<script setup lang="ts">
import { ref, computed, onBeforeUnmount, watch } from 'vue';
import { ChevronDown, Clock, Flame, Globe, Lock } from 'lucide-vue-next';
import type { Visibility } from '../api/types';
import { LIFETIME_LABEL, type LifetimeKey } from '../lib/lifetime';
import { formatDuration } from '../composables/useSnippetCountdown';

// Three pill buttons — visibility, lifetime, burn — shared between editor and
// viewer toolbars so the controls look identical before and after publish.
//
// Lifetime is special: there's no `lifetimeKey` model. The pill renders its
// label from `expires_at` — the absolute timestamp the server stamps on
// create / settings — so it always tells the truth ("expires in 14h 50m")
// instead of getting stuck on a stale preset.  When the user picks a value
// from the dropdown, we emit `lifetimeKey`; the parent translates it to
// `lifetime_seconds` and hits the API, which writes `expires_at = now() + N`.

const visibility = defineModel<Visibility>('visibility', { required: true });
const burnAfterRead = defineModel<boolean>('burnAfterRead', { required: true });

const props = defineProps<{
  mode: 'inline' | 'remote';
  disabled?: boolean;
  pending?: boolean;
  /// Absolute expiry from the server (or from the parent's local "draft"
  /// for the creation flow). `null`/`undefined` = no expiry.
  expiresAt?: string | null;
}>();

const emit = defineEmits<{
  /// Fires when the user picks a new value from a pill. The parent owns
  /// what to do with it (write a draft ref, or hit the API).
  (e: 'commit', patch: {
    visibility?: Visibility;
    lifetimeKey?: LifetimeKey;
    burnAfterRead?: boolean;
  }): void;
}>();

type OpenPopover = 'visibility' | 'lifetime' | null;
const openPopover = ref<OpenPopover>(null);

function isLocked(): boolean {
  return !!props.disabled || !!props.pending;
}

function toggle(which: 'visibility' | 'lifetime') {
  if (isLocked()) return;
  openPopover.value = openPopover.value === which ? null : which;
}

function close() {
  openPopover.value = null;
}

// Each `pickX` gates on `isLocked()` independently of `toggle()`. The
// popovers can't open while locked, so in practice pickX won't be called —
// but treating "disabled/pending" as a hard barrier at the write site means
// a future contributor wiring up a different trigger (keyboard shortcut,
// auto-suggest, etc.) inherits the lock for free.
function pickVisibility(v: Visibility) {
  if (isLocked()) return;
  close();
  if (props.mode === 'inline') visibility.value = v;
  emit('commit', { visibility: v });
}

function pickLifetime(k: LifetimeKey) {
  if (isLocked()) return;
  close();
  emit('commit', { lifetimeKey: k });
}

function toggleBurn() {
  if (isLocked()) return;
  const next = !burnAfterRead.value;
  if (props.mode === 'inline') burnAfterRead.value = next;
  emit('commit', { burnAfterRead: next });
}

// Reactive "now" tick so the lifetime label decrements (and flips to
// "expired") without a page reload. 30s granularity matches what the user
// can perceive on a pill — the more precise countdown lives in
// SnippetStatus.
const now = ref(Date.now());
let nowTimer: number | null = null;
watch(
  () => props.expiresAt ?? null,
  (exp) => {
    if (nowTimer !== null) {
      clearInterval(nowTimer);
      nowTimer = null;
    }
    if (exp) {
      nowTimer = window.setInterval(() => (now.value = Date.now()), 30_000);
    }
  },
  { immediate: true },
);
onBeforeUnmount(() => {
  if (nowTimer !== null) clearInterval(nowTimer);
});

const secondsLeft = computed<number | null>(() => {
  if (!props.expiresAt) return null;
  const t = new Date(props.expiresAt).getTime();
  if (Number.isNaN(t)) return null;
  return Math.floor((t - now.value) / 1000);
});

const isExpired = computed(() => secondsLeft.value !== null && secondsLeft.value <= 0);
const hasExpiry = computed(() => secondsLeft.value !== null);

const lifetimeBadge = computed(() => {
  if (isExpired.value) return 'expired';
  if (secondsLeft.value !== null) return formatDuration(secondsLeft.value);
  return LIFETIME_LABEL.never;
});

const pillBase = 'inline-flex items-center gap-1.5 px-2.5 py-1 rounded-sm border text-[11px] whitespace-nowrap transition-colors';

const disabledClass = computed(() =>
  props.disabled ? 'opacity-50 cursor-not-allowed' : 'hover:text-text',
);
const pendingClass = computed(() => (props.pending ? 'opacity-60 cursor-progress' : ''));

const visibilityClass = computed(() => [
  pillBase,
  visibility.value === 'private' ? 'text-warn border-warn/40' : 'text-text-muted border-border',
  disabledClass.value,
  pendingClass.value,
]);
const lifetimeClass = computed(() => [
  pillBase,
  isExpired.value
    ? 'text-danger border-danger-border bg-danger/5'
    : hasExpiry.value
      ? 'text-accent border-accent/40'
      : 'text-text-muted border-border',
  disabledClass.value,
  pendingClass.value,
]);
const burnClass = computed(() => [
  pillBase,
  burnAfterRead.value ? 'text-amber-300 border-amber-400/40 bg-amber-400/5' : 'text-text-muted border-border',
  disabledClass.value,
  pendingClass.value,
]);

defineExpose({ close });
</script>

<template>
  <div class="flex items-center gap-1.5 relative">
    <!-- visibility pill ------------------------------------------------- -->
    <div class="relative">
      <button
        type="button"
        :class="visibilityClass"
        :disabled="disabled || pending"
        :title="visibility === 'private' ? 'only signed-in users can read' : 'anyone with the link can read'"
        @click="toggle('visibility')"
      >
        <component :is="visibility === 'private' ? Lock : Globe" :size="13" />
        <span>{{ visibility }}</span>
        <ChevronDown v-if="!disabled" :size="12" class="opacity-50" />
      </button>
      <div
        v-if="openPopover === 'visibility'"
        class="absolute z-30 top-full mt-1 left-0 min-w-[200px] bg-bg-deep border border-border-strong rounded-sm shadow-lg overflow-hidden"
        @click.stop
      >
        <button
          type="button"
          class="w-full px-3 py-2 text-left text-[12px] flex items-start gap-2 hover:bg-border/40"
          :class="visibility === 'public' ? 'text-text bg-border/30' : 'text-text-muted'"
          @click="pickVisibility('public')"
        >
          <Globe :size="14" class="mt-0.5 shrink-0" />
          <span class="flex flex-col gap-0.5">
            <span>public</span>
            <span class="text-[10px] text-text-faint">anyone with the link</span>
          </span>
        </button>
        <button
          type="button"
          class="w-full px-3 py-2 text-left text-[12px] flex items-start gap-2 hover:bg-border/40"
          :class="visibility === 'private' ? 'text-text bg-border/30' : 'text-text-muted'"
          @click="pickVisibility('private')"
        >
          <Lock :size="14" class="mt-0.5 shrink-0" />
          <span class="flex flex-col gap-0.5">
            <span>private</span>
            <span class="text-[10px] text-text-faint">signed-in users only</span>
          </span>
        </button>
      </div>
    </div>

    <!-- lifetime pill --------------------------------------------------- -->
    <div class="relative">
      <button
        type="button"
        :class="lifetimeClass"
        :disabled="disabled || pending"
        :title="isExpired
          ? 'expired — pick a new lifetime to restore'
          : hasExpiry
            ? `time remaining before non-owner reads stop resolving`
            : `doesn't expire`"
        @click="toggle('lifetime')"
      >
        <Clock :size="13" />
        <span>{{ lifetimeBadge }}</span>
        <ChevronDown v-if="!disabled" :size="12" class="opacity-50" />
      </button>
      <div
        v-if="openPopover === 'lifetime'"
        class="absolute z-30 top-full mt-1 left-0 min-w-[160px] bg-bg-deep border border-border-strong rounded-sm shadow-lg overflow-hidden"
        @click.stop
      >
        <button
          v-for="key in (['never', '15m', '1h', '1d', '1w'] as LifetimeKey[])"
          :key="key"
          type="button"
          class="w-full px-3 py-2 text-left text-[12px] flex items-center gap-2 hover:bg-border/40 text-text-muted"
          @click="pickLifetime(key)"
        >
          <Clock :size="13" class="shrink-0" />
          <span>{{ LIFETIME_LABEL[key] }}</span>
        </button>
      </div>
    </div>

    <!-- burn pill (toggle, no popover) ---------------------------------- -->
    <button
      type="button"
      :class="burnClass"
      :disabled="disabled || pending"
      :title="burnAfterRead ? 'burns 15 min after first non-owner view' : 'enable to burn after first view'"
      @click="toggleBurn"
    >
      <Flame :size="13" />
      <span>burn {{ burnAfterRead ? 'on' : 'off' }}</span>
    </button>

    <!-- Click-away catcher for the visibility/lifetime popovers. -->
    <div
      v-if="openPopover !== null"
      class="fixed inset-0 z-20"
      @click="close"
    />
  </div>
</template>
