<script setup lang="ts">
import { onMounted, onUnmounted } from 'vue';
import { useRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import Shell from '../components/Shell.vue';

const auth = useAuthStore();
const router = useRouter();

let pollHandle: number | null = null;

async function poll() {
  await auth.refreshMe();
  if (auth.user?.status === 'approved') {
    router.replace('/');
  } else if (auth.user?.status === 'rejected') {
    router.replace('/rejected');
  }
}

onMounted(() => {
  pollHandle = window.setInterval(poll, 10_000);
});
onUnmounted(() => {
  if (pollHandle !== null) clearInterval(pollHandle);
});
</script>

<template>
  <Shell>
    <div class="grid place-items-center px-6 py-16">
      <div class="w-full max-w-md border border-border-strong border-l-[3px] border-l-accent p-6">
        <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · pending</div>
        <h1 class="text-base font-medium mb-2">You're in the queue.</h1>
        <p class="text-sm text-text-muted leading-relaxed">
          An admin is reviewing your request. This page checks every 10 seconds and will
          drop you on the editor as soon as you're approved.
        </p>
        <p class="text-xs text-text-muted mt-4">
          Signed in as <code class="text-text-dim">{{ auth.user?.username }}</code>.
        </p>
      </div>
    </div>
  </Shell>
</template>
