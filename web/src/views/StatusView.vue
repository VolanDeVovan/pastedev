<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { useAuthStore } from '../stores/auth';
import Shell from '../components/Shell.vue';

const auth = useAuthStore();
let pollHandle: number | null = null;
const lastChecked = ref<Date | null>(null);

async function refresh() {
  await auth.refreshSetup();
  lastChecked.value = new Date();
}

onMounted(async () => {
  await refresh();
  // 5s when everything is ok, 15s when not (instead of full exponential backoff —
  // a single polling cadence is plenty for a single-page status view).
  schedule();
});
onUnmounted(() => {
  if (pollHandle !== null) clearInterval(pollHandle);
});

const overall = computed<'ok' | 'warn' | 'err'>(() => {
  const checks = auth.setup?.checks ?? [];
  if (checks.some((c) => c.status === 'err')) return 'err';
  if (checks.some((c) => c.status === 'warn')) return 'warn';
  return 'ok';
});

function schedule() {
  if (pollHandle !== null) clearInterval(pollHandle);
  const ms = overall.value === 'ok' ? 5000 : 15000;
  pollHandle = window.setInterval(refresh, ms);
}

function statusColor(s: string): string {
  switch (s) {
    case 'ok': return 'text-accent border-accent/30';
    case 'warn': return 'text-yellow-400 border-yellow-400/30';
    case 'err': return 'text-rose-400 border-rose-400/30';
    default: return 'text-text-muted border-border-strong';
  }
}
</script>

<template>
  <Shell>
    <div class="max-w-3xl mx-auto px-6 py-10">
      <div :class="['text-[11px] tracking-widest uppercase mb-2',
        overall === 'err' ? 'text-rose-400' :
        overall === 'warn' ? 'text-yellow-400' : 'text-accent']">
        paste · status
      </div>
      <h1 class="text-lg font-medium mb-1">
        <template v-if="overall === 'ok'">All systems operational.</template>
        <template v-else-if="overall === 'warn'">Partial outage.</template>
        <template v-else>Service degraded.</template>
      </h1>
      <p class="text-xs text-text-muted mb-6">
        last checked {{ lastChecked ? lastChecked.toLocaleTimeString() : '—' }}
        · version {{ auth.setup?.version ?? '—' }}
      </p>

      <ul class="space-y-2">
        <li
          v-for="c in auth.setup?.checks ?? []"
          :key="c.id"
          :class="['border-l-[3px] bg-panel px-4 py-3', statusColor(c.status)]"
        >
          <div class="flex items-baseline justify-between">
            <span class="text-sm font-medium">{{ c.id }}</span>
            <span class="text-[11px] uppercase tracking-widest">{{ c.status }}</span>
          </div>
          <div class="text-xs text-text-muted mt-1 truncate">{{ c.detail }}</div>
        </li>
      </ul>

      <button class="mt-6 text-xs text-text-muted hover:text-text" @click="refresh">refresh now</button>
    </div>
  </Shell>
</template>
