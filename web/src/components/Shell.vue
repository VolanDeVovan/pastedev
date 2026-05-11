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
      <div class="max-w-6xl mx-auto px-4 md:px-7 h-[52px] flex items-center gap-3 md:gap-6 text-sm">
        <RouterLink to="/" class="font-bold tracking-tight text-text">pastedev</RouterLink>
        <!-- Top nav is desktop-only. Mobile relies on the bottom tab bar below
             (see mobile.jsx MTabBar) — keeping the same links in the header
             on a phone makes the bar overflow and hides the user controls. -->
        <nav class="hidden md:flex gap-6 ml-1">
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
        <div class="ml-auto flex items-center gap-2 md:gap-3 text-xs">
          <template v-if="auth.user">
            <span class="hidden md:inline text-text-muted">signed in as</span>
            <span class="text-text truncate max-w-[8rem]">{{ auth.user.username }}</span>
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
    <!-- Reserve room at the bottom for the mobile tab bar so fixed-positioned
         editor footers don't overlap it. ~52px tab + 16px safe-area inset. -->
    <main class="flex-1 pb-[68px] md:pb-0">
      <slot />
    </main>

    <!-- Mobile bottom tab bar (mirrors mobile.jsx MTabBar). Only shown when the
         viewer is approved — anonymous visitors only see the editor and view
         pages, both of which are reachable without the bar. -->
    <nav
      v-if="auth.isApproved"
      class="md:hidden fixed bottom-0 inset-x-0 z-30 bg-bg-deep border-t border-border grid text-[10px]"
      :class="auth.isAdmin ? 'grid-cols-4' : 'grid-cols-3'"
      style="padding-bottom: max(env(safe-area-inset-bottom), 8px); padding-top: 8px;"
    >
      <RouterLink
        v-for="tab in [
          { name: 'home', to: '/', label: 'new' },
          { name: 'dashboard', to: '/dashboard', label: 'snippets' },
          { name: 'keys', to: '/keys', label: 'keys' },
          ...(auth.isAdmin ? [{ name: 'admin', to: '/admin', label: 'admin' }] : []),
        ]"
        :key="tab.name"
        :to="tab.to"
        :class="[
          'relative text-center py-1',
          isActive(tab.name) ? 'text-text' : 'text-text-muted',
        ]"
      >
        <span
          v-if="isActive(tab.name)"
          class="absolute top-0 left-1/2 -translate-x-1/2 w-6 h-0.5 bg-accent rounded-full"
        />
        {{ tab.label }}
      </RouterLink>
    </nav>
  </div>
</template>
