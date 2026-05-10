<script setup lang="ts">
import { useRouter } from 'vue-router';
import Shell from '../components/Shell.vue';
import { useAuthStore } from '../stores/auth';

const auth = useAuthStore();
const router = useRouter();

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
        <div class="text-[10px] tracking-widest uppercase text-danger mb-3">access denied</div>
        <h1 class="text-[22px] tracking-tight mb-2.5">your request was declined</h1>
        <p class="text-[13px] text-text-muted leading-relaxed mb-7">
          an admin reviewed and rejected the registration for
          <span class="text-text">{{ auth.user?.username }}</span>.
          if you think this is a mistake, reach out to the operator out-of-band.
        </p>
        <button class="text-text-muted hover:text-text border border-border-strong rounded-sm px-3 py-1.5 text-[12px]" @click="signOut">sign out</button>
      </div>
    </div>
  </Shell>
</template>
