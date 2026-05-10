<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { useRouter } from 'vue-router';
import { useAuthStore } from '../stores/auth';
import FormField from '../components/FormField.vue';
import { HttpError } from '../api';

const auth = useAuthStore();
const router = useRouter();

const step = ref<'check' | 'admin'>('check');
const username = ref('');
const email = ref('');
const password = ref('');
const error = ref<string | null>(null);
const submitting = ref(false);

let pollHandle: number | null = null;

async function poll() {
  await auth.refreshSetup();
  if (!auth.needsSetup) {
    router.replace('/');
  }
}

onMounted(async () => {
  await auth.refreshSetup();
  if (!auth.needsSetup) {
    router.replace('/');
    return;
  }
  pollHandle = window.setInterval(poll, 2000);
});

onUnmounted(() => {
  if (pollHandle !== null) clearInterval(pollHandle);
});

const allChecksOk = computed(
  () => auth.setup?.checks.every((c) => c.status === 'ok') ?? false,
);

async function submit() {
  error.value = null;
  submitting.value = true;
  try {
    await auth.setupAdmin({
      username: username.value.trim(),
      email: email.value.trim() || undefined,
      password: password.value,
    });
    router.replace('/dashboard');
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'something went wrong';
  } finally {
    submitting.value = false;
  }
}

function statusColor(s: string): string {
  switch (s) {
    case 'ok': return 'text-accent';
    case 'warn': return 'text-yellow-400';
    case 'err': return 'text-rose-400';
    default: return 'text-text-muted';
  }
}
</script>

<template>
  <div class="min-h-screen grid place-items-center px-6 py-12">
    <div class="w-full max-w-xl">
      <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · setup</div>
      <h1 class="text-xl font-medium mb-2">Welcome. Let's get this instance up.</h1>
      <p class="text-sm text-text-muted mb-8">
        The users table is empty. Confirm the environment looks healthy, then create the first admin.
      </p>

      <ol class="flex gap-6 text-[11px] uppercase tracking-widest mb-6">
        <li :class="step === 'check' ? 'text-accent' : 'text-text-muted'">01 · environment</li>
        <li :class="step === 'admin' ? 'text-accent' : 'text-text-muted'">02 · root admin</li>
      </ol>

      <section v-if="step === 'check'" class="border border-border-strong border-l-[3px] border-l-accent p-6 space-y-3">
        <p class="text-xs text-text-muted">Each check is read-only; the SPA polls until everything is green.</p>
        <ul class="space-y-2">
          <li v-for="c in auth.setup?.checks ?? []" :key="c.id" class="flex items-baseline gap-3">
            <span :class="[statusColor(c.status), 'w-12 text-[11px] uppercase tracking-widest']">{{ c.status }}</span>
            <span class="text-sm w-32">{{ c.id }}</span>
            <span class="text-xs text-text-muted truncate">{{ c.detail }}</span>
          </li>
        </ul>
        <div class="pt-3 flex justify-end">
          <button
            class="text-sm border border-accent text-accent px-4 py-2 hover:bg-accent hover:text-bg-deep transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
            :disabled="!allChecksOk"
            @click="step = 'admin'"
          >continue →</button>
        </div>
      </section>

      <section v-else class="border border-border-strong border-l-[3px] border-l-accent p-6">
        <form class="space-y-4" @submit.prevent="submit">
          <FormField v-model="username" label="username" autocomplete="username" required placeholder="kirill" hint="lowercase letters, digits, _ . - · 3 to 40 chars" />
          <FormField v-model="email" label="email · optional" type="email" autocomplete="email" placeholder="kirill@dev.su" />
          <FormField v-model="password" label="password" type="password" autocomplete="new-password" required hint="12+ characters" />
          <div v-if="error" class="text-sm text-rose-400">{{ error }}</div>
          <div class="flex justify-between items-center pt-2">
            <button type="button" class="text-sm text-text-muted hover:text-text" @click="step = 'check'">← back</button>
            <button
              type="submit"
              :disabled="submitting"
              class="text-sm border border-accent text-accent px-4 py-2 hover:bg-accent hover:text-bg-deep transition-colors disabled:opacity-30"
            >{{ submitting ? 'creating…' : 'create admin' }}</button>
          </div>
        </form>
      </section>
    </div>
  </div>
</template>
