<script setup lang="ts">
import { config } from '../config';
import { ref, onMounted } from 'vue';

const health = ref<string>('checking…');

onMounted(async () => {
  try {
    const r = await fetch(`${config.apiBaseUrl}/api/v1/health`, { credentials: 'include' });
    const body = await r.json();
    health.value = body.ok ? 'ok' : 'err';
  } catch (e) {
    health.value = 'unreachable';
  }
});
</script>

<template>
  <main class="min-h-screen grid place-items-center">
    <div class="border border-border-strong border-l-4 border-l-accent px-8 py-6 max-w-xl">
      <div class="text-[11px] tracking-widest uppercase text-accent">paste · scaffold</div>
      <h1 class="text-base font-medium mt-2">paste-server is up.</h1>
      <p class="text-sm text-text-muted mt-3 leading-relaxed">
        This is the phase-0 scaffold. The Vue SPA is loading; the editor, auth,
        and admin views land in later phases.
      </p>
      <div class="mt-4 flex gap-3 text-xs text-text-muted">
        <span>api base: <code class="text-text-dim">{{ config.apiBaseUrl || '(same-origin)' }}</code></span>
        <span>·</span>
        <span>health: <code class="text-text-dim">{{ health }}</code></span>
      </div>
    </div>
  </main>
</template>
