<script setup lang="ts">
import { computed, toRef } from 'vue';
import { Clock } from 'lucide-vue-next';
import type { Snippet } from '../api';
import { useSnippetCountdown } from '../composables/useSnippetCountdown';

// One chip: the live time-remaining countdown.
//
// Everything else (visibility, burn-after-read, "expired") is already
// surfaced by the mutable `PolicyBar` pills above and the explanatory
// banner below — duplicating them here was just noise. The ticking
// `m s` countdown is the one piece of state PolicyBar doesn't show (its
// label is static; this one updates every second).

const props = defineProps<{ snippet: Snippet }>();
const snippetRef = toRef(props, 'snippet');
const { label, expired, burnArmed } = useSnippetCountdown(snippetRef);

// Only render while there's something live to count or a "burn is armed"
// hint to show. Expired is handled by the warning banner the parent
// renders separately, so we hide ourselves in that state.
const visible = computed(() => {
  if (expired.value) return false;
  if (burnArmed.value) return true;
  return props.snippet.expires_at != null;
});

const chipClass = computed(() =>
  burnArmed.value
    ? 'text-warn border-warn/40'
    : 'text-accent border-accent/40',
);
</script>

<template>
  <div v-if="visible" class="flex flex-wrap items-center gap-2 mb-3 text-[11px]">
    <span
      :class="[
        'inline-flex items-center gap-1 px-2 py-0.5 rounded-sm border tabular-nums',
        chipClass,
      ]"
      :title="burnArmed
        ? 'first non-owner view starts a 15-minute fuse'
        : 'time until this snippet stops being readable'"
    >
      <Clock :size="11" />
      <span>{{ label }}</span>
    </span>
  </div>
</template>
