<script setup lang="ts">
import { Flame, Globe, Lock } from 'lucide-vue-next';
import type { Visibility } from '../api/types';
import type { LifetimeKey } from '../lib/lifetime';
import Modal from './Modal.vue';

// Shared state owned by the editor — bound through defineModel so mobile
// (modal) and desktop (inline toolbar) drive the exact same refs without
// having to forward updates through emits.
const open = defineModel<boolean>('open', { required: true });
const visibility = defineModel<Visibility>('visibility', { required: true });
const lifetimeKey = defineModel<LifetimeKey>('lifetimeKey', { required: true });
const burnAfterRead = defineModel<boolean>('burnAfterRead', { required: true });

defineProps<{ submitting?: boolean }>();
const emit = defineEmits<{ (e: 'publish'): void }>();

// Segmented control labels — short enough to fit on a phone, with the
// inline label baked into the button text so we don't waste a row on a
// "Expires:" caption above. Typed against LifetimeKey so a stray label
// can't drift away from the canonical set in lib/lifetime.ts.
const LIFETIME_OPTIONS: { key: LifetimeKey; label: string }[] = [
  { key: 'never', label: 'never' },
  { key: '15m', label: '15m' },
  { key: '1h', label: '1h' },
  { key: '1d', label: '1d' },
  { key: '1w', label: '1w' },
];

function publish() {
  emit('publish');
}
</script>

<template>
  <Modal v-model:open="open" title="publish options">
    <div class="space-y-5">
      <!-- Visibility — two-up segmented control. We pad the descriptive
           subtext below each option rather than relying on tooltips so the
           difference is obvious without long-press. -->
      <div>
        <div class="text-[10px] uppercase tracking-widest text-text-muted mb-1.5">visibility</div>
        <div class="grid grid-cols-2 gap-1.5">
          <button
            type="button"
            :class="[
              'flex flex-col items-start gap-1 px-3 py-2 rounded-sm border text-left transition-colors',
              visibility === 'public'
                ? 'bg-border text-text border-border-strong'
                : 'border-border text-text-muted hover:text-text',
            ]"
            @click="visibility = 'public'"
          >
            <span class="inline-flex items-center gap-1.5 text-[12px]"><Globe :size="13" /> public</span>
            <span class="text-[10px] text-text-faint">anyone with the link</span>
          </button>
          <button
            type="button"
            :class="[
              'flex flex-col items-start gap-1 px-3 py-2 rounded-sm border text-left transition-colors',
              visibility === 'private'
                ? 'bg-border text-text border-border-strong'
                : 'border-border text-text-muted hover:text-text',
            ]"
            @click="visibility = 'private'"
          >
            <span class="inline-flex items-center gap-1.5 text-[12px]"><Lock :size="13" /> private</span>
            <span class="text-[10px] text-text-faint">signed-in users only</span>
          </button>
        </div>
      </div>

      <!-- Expires — 5-up segmented control. Buttons share the row so the
           selected value is obvious at a glance; no extra "Selected: 1h"
           caption needed. -->
      <div>
        <div class="text-[10px] uppercase tracking-widest text-text-muted mb-1.5">expires</div>
        <div class="grid grid-cols-5 gap-1">
          <button
            v-for="opt in LIFETIME_OPTIONS"
            :key="opt.key"
            type="button"
            :class="[
              'px-2 py-1.5 rounded-sm border text-[12px] transition-colors',
              lifetimeKey === opt.key
                ? 'bg-accent text-bg-deep border-accent'
                : 'border-border text-text-muted hover:text-text',
            ]"
            @click="lifetimeKey = opt.key"
          >{{ opt.label }}</button>
        </div>
        <p class="text-[10px] text-text-faint mt-1.5">
          counted from creation. <span v-if="lifetimeKey === 'never'">snippet stays alive until you delete it.</span>
        </p>
      </div>

      <!-- Burn-after-read — a single row toggle. Tap the entire row to flip;
           the explanation under the label is the whole reason this option
           exists and shouldn't be tooltip-only on a phone. -->
      <button
        type="button"
        :class="[
          'w-full flex items-center justify-between px-3 py-2.5 rounded-sm border transition-colors text-left',
          burnAfterRead
            ? 'bg-amber-400/10 border-amber-400/50 text-amber-200'
            : 'border-border text-text-muted hover:text-text',
        ]"
        @click="burnAfterRead = !burnAfterRead"
      >
        <span class="flex flex-col gap-0.5">
          <span class="inline-flex items-center gap-1.5 text-[12px]"><Flame :size="13" /> burn after read</span>
          <span class="text-[10px] text-text-faint">expires 15 min after first non-owner view</span>
        </span>
        <span
          :class="[
            'inline-block w-9 h-5 rounded-full border transition-colors relative shrink-0',
            burnAfterRead ? 'bg-amber-400/40 border-amber-400/60' : 'bg-bg border-border-strong',
          ]"
          aria-hidden="true"
        >
          <span
            :class="[
              'absolute top-0.5 w-4 h-4 rounded-full transition-all',
              burnAfterRead ? 'left-[18px] bg-amber-300' : 'left-0.5 bg-text-muted',
            ]"
          />
        </span>
      </button>
    </div>

    <template #actions>
      <button
        type="button"
        class="text-text-muted hover:text-text px-3 py-2 text-[12px]"
        :disabled="submitting"
        @click="open = false"
      >cancel</button>
      <button
        type="button"
        class="bg-accent text-bg-deep font-semibold px-4 py-2 text-[12px] rounded-sm hover:opacity-90 disabled:opacity-50"
        :disabled="submitting"
        @click="publish"
      >{{ submitting ? 'publishing…' : 'publish' }}</button>
    </template>
  </Modal>
</template>
