<script setup lang="ts">
import { ref } from 'vue';
import { useRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import FormField from '../components/FormField.vue';
import Shell from '../components/Shell.vue';
import { HttpError } from '../api';

const auth = useAuthStore();
const router = useRouter();

const username = ref('');
const email = ref('');
const password = ref('');
const reason = ref('');
const error = ref<string | null>(null);
const submitting = ref(false);

async function submit() {
  error.value = null;
  if (reason.value.trim().length < 10) {
    error.value = 'tell the admin a bit about why you want access (10+ chars)';
    return;
  }
  submitting.value = true;
  try {
    await auth.register({
      username: username.value.trim(),
      email: email.value.trim() || undefined,
      password: password.value,
      reason: reason.value,
    });
    router.replace('/pending');
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
        <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · register</div>
        <h1 class="text-xl font-medium mb-2">Request access.</h1>
        <p class="text-sm text-text-muted mb-6">An admin reviews and approves new accounts.</p>
        <form class="border border-border-strong border-l-[3px] border-l-accent p-6 space-y-4" @submit.prevent="submit">
          <FormField v-model="username" label="username" autocomplete="username" required hint="lowercase letters, digits, _ . -" />
          <FormField v-model="email" label="email · optional" type="email" autocomplete="email" />
          <FormField v-model="password" label="password" type="password" autocomplete="new-password" required hint="12+ chars" />
          <label class="block">
            <span class="text-[11px] uppercase tracking-widest text-text-muted">why do you want access?</span>
            <textarea
              v-model="reason"
              rows="4"
              required
              minlength="10"
              maxlength="500"
              class="mt-1.5 w-full bg-panel border border-border-strong px-3 py-2 text-sm focus:outline-none focus:border-accent transition-colors resize-none"
              placeholder="i write docs and want a place to share them with the team"
            />
          </label>
          <div v-if="error" class="text-sm text-rose-400">{{ error }}</div>
          <button
            type="submit"
            :disabled="submitting"
            class="w-full text-sm border border-accent text-accent px-4 py-2 hover:bg-accent hover:text-bg-deep transition-colors disabled:opacity-30"
          >{{ submitting ? 'submitting…' : 'request access' }}</button>
        </form>
      </div>
    </div>
  </Shell>
</template>
