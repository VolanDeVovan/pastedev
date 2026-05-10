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
    <div class="flex justify-center pt-20 px-6">
      <div class="w-[420px]">
        <h1 class="text-[22px] tracking-tight mb-1.5">request access</h1>
        <p class="text-[12px] text-text-muted leading-relaxed mb-7">
          registration is reviewed by an admin. once approved, you can publish
          snippets through the web app or via API keys.
        </p>

        <form @submit.prevent="submit">
          <FormField v-model="username" label="username" autocomplete="username" required hint="3–40 chars · lowercase" />
          <FormField v-model="email" label="email · optional" type="email" autocomplete="email" />
          <FormField v-model="password" label="password" type="password" autocomplete="new-password" required hint="12+ chars" />
          <FormField v-model="reason" label="why do you want access" :rows="4" required />

          <div v-if="error" class="text-[12px] text-danger mb-3">{{ error }}</div>

          <button
            type="submit"
            :disabled="submitting"
            class="w-full bg-accent text-bg-deep font-semibold py-2.5 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30 transition-opacity"
          >{{ submitting ? 'submitting…' : 'submit request →' }}</button>
        </form>

        <p class="text-center text-[12px] text-text-muted mt-5">
          already approved?
          <RouterLink to="/signin" class="text-accent hover:underline">sign in</RouterLink>
        </p>
      </div>
    </div>
  </Shell>
</template>
