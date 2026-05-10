<script setup lang="ts">
import { useRoute, useRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';

const auth = useAuthStore();
const router = useRouter();
const route = useRoute();

function isActive(...names: string[]): boolean {
  return names.includes(route.name as string);
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
  <div class="min-h-screen flex flex-col">
    <header class="border-b border-border">
      <div class="max-w-6xl mx-auto px-7 h-[52px] flex items-center gap-6 text-sm">
        <RouterLink to="/" class="font-bold tracking-tight text-text">pastedev</RouterLink>
        <nav class="flex gap-6 ml-1">
          <RouterLink
            v-if="auth.isApproved"
            to="/"
            :class="[
              'pb-0.5 border-b',
              isActive('home') ? 'text-text border-accent' : 'text-text-muted border-transparent hover:text-text'
            ]"
          >new</RouterLink>
          <RouterLink
            v-if="auth.isApproved"
            to="/dashboard"
            :class="[
              'pb-0.5 border-b',
              isActive('dashboard') ? 'text-text border-accent' : 'text-text-muted border-transparent hover:text-text'
            ]"
          >my snippets</RouterLink>
          <RouterLink
            v-if="auth.isApproved"
            to="/keys"
            :class="[
              'pb-0.5 border-b',
              isActive('keys') ? 'text-text border-accent' : 'text-text-muted border-transparent hover:text-text'
            ]"
          >api keys</RouterLink>
          <RouterLink
            v-if="auth.isAdmin"
            to="/admin"
            :class="[
              'pb-0.5 border-b',
              isActive('admin') ? 'text-text border-accent' : 'text-text-muted border-transparent hover:text-text'
            ]"
          >admin</RouterLink>
        </nav>
        <div class="ml-auto flex items-center gap-3 text-xs">
          <template v-if="auth.user">
            <span class="text-text-muted">signed in as</span>
            <span class="text-text">{{ auth.user.username }}</span>
            <span class="w-px h-3.5 bg-border-strong" />
            <button class="text-text-muted hover:text-text" @click="signOut">sign out</button>
          </template>
          <template v-else>
            <RouterLink to="/signin" class="text-text-muted hover:text-text">sign in</RouterLink>
            <RouterLink
              to="/register"
              class="bg-accent text-bg-deep px-3 py-1 font-semibold rounded-sm hover:opacity-90 transition-opacity"
            >register</RouterLink>
          </template>
        </div>
      </div>
    </header>
    <main class="flex-1">
      <slot />
    </main>
  </div>
</template>
