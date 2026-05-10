<script setup lang="ts">
import { ref } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import FormField from '../components/FormField.vue';
import Shell from '../components/Shell.vue';
import { HttpError } from '../api';

const auth = useAuthStore();
const router = useRouter();
const route = useRoute();

const username = ref('');
const password = ref('');
const error = ref<string | null>(null);
const submitting = ref(false);

async function submit() {
  error.value = null;
  submitting.value = true;
  try {
    await auth.login({ username: username.value.trim(), password: password.value });
    const next = (route.query.next as string | undefined) ?? '/';
    router.replace(next);
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'something went wrong';
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <Shell>
    <div class="grid place-items-center px-6 py-16">
      <div class="w-full max-w-md">
        <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · sign in</div>
        <h1 class="text-xl font-medium mb-6">Welcome back.</h1>
        <form class="border border-border-strong border-l-[3px] border-l-accent p-6 space-y-4" @submit.prevent="submit">
          <FormField v-model="username" label="username" autocomplete="username" required />
          <FormField v-model="password" label="password" type="password" autocomplete="current-password" required />
          <div v-if="error" class="text-sm text-rose-400">{{ error }}</div>
          <button
            type="submit"
            :disabled="submitting"
            class="w-full text-sm border border-accent text-accent px-4 py-2 hover:bg-accent hover:text-bg-deep transition-colors disabled:opacity-30"
          >{{ submitting ? 'signing in…' : 'sign in' }}</button>
        </form>
        <p class="text-xs text-text-muted mt-6">
          No account yet?
          <RouterLink class="text-accent hover:underline" to="/register">request access</RouterLink>
        </p>
      </div>
    </div>
  </Shell>
</template>
