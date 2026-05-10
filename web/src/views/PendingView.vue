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
  if (auth.user?.status === 'approved') router.replace('/');
  else if (auth.user?.status === 'rejected') router.replace('/rejected');
}

onMounted(() => {
  pollHandle = window.setInterval(poll, 10_000);
});
onUnmounted(() => {
  if (pollHandle !== null) clearInterval(pollHandle);
});

const submittedAt = new Date().toLocaleDateString(undefined, {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
});

async function checkStatus() {
  await auth.refreshMe();
}

async function signOut() {
  try {
    await auth.logout();
  } finally {
    router.push('/signin');
  }
}
</script>

<template>
  <Shell>
    <div class="flex justify-center pt-32 px-6">
      <div class="w-[460px] text-center">
        <div class="inline-flex gap-1.5 mb-5">
          <span class="w-2 h-2 rounded-full bg-accent inline-block animate-[paste-pulse_1.4s_infinite]" />
          <span class="w-2 h-2 rounded-full bg-accent inline-block animate-[paste-pulse_1.4s_infinite] [animation-delay:200ms]" />
          <span class="w-2 h-2 rounded-full bg-accent inline-block animate-[paste-pulse_1.4s_infinite] [animation-delay:400ms]" />
        </div>
        <h1 class="text-[22px] tracking-tight mb-2.5">waiting for approval</h1>
        <p class="text-[13px] text-text-muted leading-relaxed mb-7">
          your request was submitted on
          <span class="text-text">{{ submittedAt }}</span>.
          an admin will review it shortly. this page checks every 10 seconds and
          drops you on the editor as soon as you're approved.
        </p>
        <div class="flex gap-2 justify-center">
          <button class="text-text-muted hover:text-text border border-border-strong rounded-sm px-3 py-1.5 text-[12px]" @click="checkStatus">check status</button>
          <button class="text-text-muted hover:text-text border border-border-strong rounded-sm px-3 py-1.5 text-[12px]" @click="signOut">sign out</button>
        </div>
      </div>
    </div>
  </Shell>
</template>
