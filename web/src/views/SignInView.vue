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
    <div class="flex justify-center pt-20 px-6">
      <div class="w-[380px]">
        <h1 class="text-[22px] tracking-tight mb-1.5">sign in</h1>
        <p class="text-[12px] text-text-muted leading-relaxed mb-7">
          use your credentials to continue.
        </p>

        <form @submit.prevent="submit">
          <FormField v-model="username" label="username" autocomplete="username" required />
          <FormField v-model="password" label="password" type="password" autocomplete="current-password" required />

          <div v-if="error" class="text-[12px] text-danger mb-3">{{ error }}</div>

          <button
            type="submit"
            :disabled="submitting"
            class="w-full bg-accent text-bg-deep font-semibold py-2.5 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30 transition-opacity"
          >{{ submitting ? 'signing in…' : 'continue →' }}</button>
        </form>

        <p class="text-center text-[12px] text-text-muted mt-5">
          no account?
          <RouterLink to="/register" class="text-accent hover:underline">request access</RouterLink>
        </p>
      </div>
    </div>
  </Shell>
</template>
