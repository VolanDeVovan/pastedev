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
  if (!auth.needsSetup) router.replace('/');
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

const allOk = computed(() => auth.setup?.checks.every((c) => c.status === 'ok') ?? false);

async function submit() {
  error.value = null;
  submitting.value = true;
  try {
    await auth.setupAdmin({
      username: username.value.trim(),
      email: email.value.trim() || undefined,
      password: password.value,
    });
    router.replace('/');
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'something went wrong';
  } finally {
    submitting.value = false;
  }
}

function dotColor(s: string): string {
  switch (s) {
    case 'ok': return 'bg-accent';
    case 'warn': return 'bg-warn';
    case 'err': return 'bg-danger';
    default: return 'bg-text-muted animate-[paste-pulse_1.4s_infinite]';
  }
}
function labelColor(s: string): string {
  switch (s) {
    case 'ok': return 'text-accent';
    case 'warn': return 'text-warn';
    case 'err': return 'text-danger';
    default: return 'text-text-muted';
  }
}
</script>

<template>
  <div class="min-h-screen flex flex-col">
    <div class="flex items-center justify-between px-4 md:px-7 py-3 md:py-3.5 border-b border-border text-[13px]">
      <div class="flex items-center gap-2 md:gap-3.5">
        <span class="font-bold tracking-tight">pastedev</span>
        <span class="px-2 py-0.5 text-[10px] tracking-widest uppercase text-warn border border-warn/40 rounded-sm">first run</span>
      </div>
      <div class="text-[11px] text-text-muted">v{{ auth.setup?.version ?? '0.0.0' }}</div>
    </div>

    <!-- Mobile-only step strip — collapses the desktop sidebar's two-step
         indicator into a horizontal bar at the top of the page. -->
    <div class="md:hidden flex items-center gap-3 px-4 py-3 border-b border-border bg-bg-deep text-[12px]">
      <div class="flex items-center gap-2">
        <span :class="[
          'w-[22px] h-[22px] rounded-full inline-flex items-center justify-center text-[11px] border shrink-0',
          step === 'check' ? 'border-accent text-accent' : 'border-accent/60 text-accent bg-accent/15'
        ]">{{ step === 'check' ? '1' : '✓' }}</span>
        <span :class="step === 'check' ? 'text-text' : 'text-text-dim'">env check</span>
      </div>
      <span class="flex-1 h-px bg-border" />
      <div class="flex items-center gap-2">
        <span :class="[
          'w-[22px] h-[22px] rounded-full inline-flex items-center justify-center text-[11px] border shrink-0',
          step === 'admin' ? 'border-accent text-accent' : 'border-border-strong text-text-muted'
        ]">2</span>
        <span :class="step === 'admin' ? 'text-text' : 'text-text-muted'">root admin</span>
      </div>
    </div>

    <div class="flex-1 md:grid md:grid-cols-[260px_1fr]">
      <aside class="hidden md:block px-6 py-8 border-r border-border bg-bg-deep">
        <div class="text-[11px] tracking-widest uppercase text-text-muted mb-5">setup</div>
        <div class="flex flex-col gap-4">
          <div class="flex items-center gap-2.5">
            <span :class="[
              'w-[22px] h-[22px] rounded-full inline-flex items-center justify-center text-[11px] border',
              step === 'check' ? 'border-accent text-accent' : 'border-accent/60 text-accent bg-accent/15'
            ]">{{ step === 'check' ? '1' : '✓' }}</span>
            <span :class="['text-[12px]', step === 'check' ? 'text-text' : 'text-text-dim']">environment check</span>
          </div>
          <div class="flex items-center gap-2.5">
            <span :class="[
              'w-[22px] h-[22px] rounded-full inline-flex items-center justify-center text-[11px] border',
              step === 'admin' ? 'border-accent text-accent' : 'border-border-strong text-text-muted'
            ]">2</span>
            <span :class="['text-[12px]', step === 'admin' ? 'text-text' : 'text-text-muted']">create root admin</span>
          </div>
        </div>
        <div class="mt-8 text-[11px] text-text-muted leading-relaxed">
          this is a one-time setup. the first user becomes the root admin and is
          approved automatically. afterwards, all new registrations require admin review.
          <div class="mt-3.5 pt-3.5 border-t border-border">
            instance settings come from environment variables — see
            <span class="text-text">.env.example</span> in the repo.
          </div>
        </div>
      </aside>

      <section v-if="step === 'check'" class="px-4 md:px-10 py-5 md:py-8 max-w-2xl">
        <h1 class="text-[20px] md:text-[22px] tracking-tight mb-1.5">environment check</h1>
        <p class="text-[12px] text-text-muted leading-relaxed mb-6 max-w-md">
          each check is read-only. the page polls until everything is green — then
          you can move on.
        </p>

        <div class="border border-border bg-bg-deep rounded-sm">
          <div
            v-for="(c, i) in auth.setup?.checks ?? []"
            :key="c.id"
            :class="['grid grid-cols-[20px_1fr_auto] gap-3 items-center px-3.5 py-3', i ? 'border-t border-border' : '']"
          >
            <span :class="['w-2 h-2 rounded-full inline-block', dotColor(c.status), c.status === 'ok' ? 'shadow-[0_0_8px_var(--color-accent)]' : '']" />
            <div>
              <div class="text-[13px] text-text">{{ c.id }}</div>
              <div class="text-[11px] text-text-muted mt-0.5">{{ c.detail }}</div>
            </div>
            <span :class="['text-[11px] tracking-widest uppercase', labelColor(c.status)]">{{ c.status }}</span>
          </div>
        </div>

        <div class="flex justify-end mt-6">
          <button
            class="bg-accent text-bg-deep font-semibold px-5 py-2.5 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30 disabled:cursor-not-allowed transition-opacity"
            :disabled="!allOk"
            @click="step = 'admin'"
          >continue →</button>
        </div>
      </section>

      <section v-else class="px-4 md:px-10 py-5 md:py-8 max-w-xl">
        <h1 class="text-[20px] md:text-[22px] tracking-tight mb-1.5">create root admin</h1>
        <p class="text-[12px] text-text-muted leading-relaxed mb-6 max-w-md">
          this account skips the approval queue. it can approve other registrations,
          mint api keys, and access the admin panel.
        </p>

        <form @submit.prevent="submit">
          <FormField v-model="username" label="username" autocomplete="username" required hint="3–40 chars · lowercase · _ . -" />
          <FormField v-model="email" label="email · optional" type="email" autocomplete="email" />
          <FormField v-model="password" label="password" type="password" autocomplete="new-password" required hint="8+ chars" />

          <div class="bg-accent/5 border border-accent/30 rounded-sm px-3.5 py-3 mt-2 mb-5 text-[12px] text-text-dim leading-relaxed">
            <div class="flex items-center gap-2 mb-1">
              <span class="w-2 h-2 rounded-full bg-accent inline-block shadow-[0_0_8px_var(--color-accent)]" />
              <span class="text-accent text-[10px] tracking-widest uppercase">auto-approve</span>
            </div>
            you are user #1. registration approval is bypassed for the bootstrap admin.
          </div>

          <div v-if="error" class="text-[12px] text-danger mb-3">{{ error }}</div>

          <div class="flex justify-between items-center">
            <button type="button" class="text-[12px] text-text-muted hover:text-text" @click="step = 'check'">← back</button>
            <button
              type="submit"
              :disabled="submitting"
              class="bg-accent text-bg-deep font-semibold px-5 py-2.5 text-[13px] rounded-sm hover:opacity-90 disabled:opacity-30 transition-opacity"
            >{{ submitting ? 'creating…' : 'create admin & finish →' }}</button>
          </div>
        </form>
      </section>
    </div>
  </div>
</template>
