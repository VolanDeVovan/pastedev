<script setup lang="ts">
import { useAuthStore } from '../stores/auth';
import Shell from '../components/Shell.vue';

const auth = useAuthStore();
</script>

<template>
  <Shell>
    <div class="max-w-3xl mx-auto px-6 py-16">
      <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste</div>
      <h1 class="text-xl font-medium mb-3">Editor lands in phase 2.</h1>
      <p class="text-sm text-text-muted leading-relaxed max-w-xl">
        Auth, registration, and admin gating work today (phase 1).
        Code/markdown/html publishing and the dashboard come next.
      </p>
      <p v-if="auth.user" class="text-xs text-text-muted mt-6">
        Signed in as <code class="text-text-dim">{{ auth.user.username }}</code>
        · status <code class="text-text-dim">{{ auth.user.status }}</code>
        <span v-if="auth.isAdmin"> · role <code class="text-text-dim">admin</code></span>
      </p>
    </div>
  </Shell>
</template>
