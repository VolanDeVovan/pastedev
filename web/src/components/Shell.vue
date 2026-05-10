<script setup lang="ts">
import { useRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import { config } from '../config';

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
  <div class="min-h-screen flex flex-col">
    <header class="border-b border-border-strong">
      <div class="max-w-6xl mx-auto px-6 h-14 flex items-center gap-6">
        <RouterLink to="/" class="flex items-center gap-2 text-text">
          <span class="font-medium tracking-tight">{{ config.appName }}</span>
          <span class="text-text-muted text-[11px] uppercase tracking-widest">snippet host</span>
        </RouterLink>
        <nav class="hidden sm:flex gap-5 text-sm text-text-muted">
          <RouterLink v-if="auth.isApproved" to="/dashboard" class="hover:text-text">dashboard</RouterLink>
          <RouterLink v-if="auth.isApproved" to="/keys" class="hover:text-text">keys</RouterLink>
          <RouterLink v-if="auth.isAdmin" to="/admin" class="hover:text-text">admin</RouterLink>
        </nav>
        <div class="ml-auto text-sm text-text-muted flex items-center gap-3">
          <template v-if="auth.user">
            <span>{{ auth.user.username }}</span>
            <button class="text-accent hover:underline" @click="signOut">sign out</button>
          </template>
          <template v-else>
            <RouterLink to="/signin" class="hover:text-text">sign in</RouterLink>
            <RouterLink to="/register" class="hover:text-text">register</RouterLink>
          </template>
        </div>
      </div>
    </header>
    <main class="flex-1">
      <slot />
    </main>
  </div>
</template>
