<script setup lang="ts">
import Shell from '../components/Shell.vue';
import { useAuthStore } from '../stores/auth';

const auth = useAuthStore();
</script>

<template>
  <Shell>
    <div class="grid place-items-center px-6 py-16">
      <div class="w-full max-w-md border border-border-strong border-l-[3px] border-l-rose-500 p-6">
        <div class="text-[11px] tracking-widest uppercase text-rose-400 mb-2">paste · rejected</div>
        <h1 class="text-base font-medium mb-2">Your request was declined.</h1>
        <p class="text-sm text-text-muted leading-relaxed">
          An admin reviewed and rejected your account. If you think this was a mistake,
          reach out to the operator out-of-band.
        </p>
        <p class="text-xs text-text-muted mt-4">
          Signed in as <code class="text-text-dim">{{ auth.user?.username }}</code>.
        </p>
      </div>
    </div>
  </Shell>
</template>
