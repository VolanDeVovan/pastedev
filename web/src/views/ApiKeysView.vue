<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { config } from '../config';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import { HttpError } from '../api';
import type { Scope } from '../api/types';

interface KeyView {
  id: string;
  name: string;
  prefix: string;
  scopes: Scope[];
  created_at: string;
  last_used_at: string | null;
  revoked_at: string | null;
}
interface KeyMinted extends KeyView {
  token: string;
}

const items = ref<KeyView[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const showForm = ref(false);

const newName = ref('');
const newScopes = ref<Record<Scope, boolean>>({ publish: true, read: false, delete: false });
const minted = ref<KeyMinted | null>(null);

const showRevoke = ref(false);
const revokeTarget = ref<KeyView | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const r = await fetch(`${config.apiBaseUrl}/api/v1/keys`, { credentials: 'include' });
    if (!r.ok) throw new HttpError(r.status, (await r.json()).error);
    const data = (await r.json()) as { items: KeyView[] };
    items.value = data.items;
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'failed';
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

async function create() {
  error.value = null;
  const scopes: Scope[] = (Object.keys(newScopes.value) as Scope[]).filter((s) => newScopes.value[s]);
  try {
    const r = await fetch(`${config.apiBaseUrl}/api/v1/keys`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ name: newName.value.trim(), scopes }),
    });
    if (!r.ok) throw new HttpError(r.status, (await r.json()).error);
    minted.value = (await r.json()) as KeyMinted;
    newName.value = '';
    newScopes.value = { publish: true, read: false, delete: false };
    showForm.value = false;
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'create failed';
  }
}

async function copyToken() {
  if (minted.value) await navigator.clipboard.writeText(minted.value.token);
}

function askRevoke(k: KeyView) {
  revokeTarget.value = k;
  showRevoke.value = true;
}

// "rotate" doesn't have a dedicated endpoint yet — surface the same generate
// form so the user creates a fresh key, then revokes the old one manually.
function rotate() {
  showForm.value = true;
}

async function confirmRevoke() {
  const k = revokeTarget.value;
  if (!k) return;
  showRevoke.value = false;
  try {
    const r = await fetch(`${config.apiBaseUrl}/api/v1/keys/${k.id}`, {
      method: 'DELETE',
      credentials: 'include',
    });
    if (!r.ok) throw new HttpError(r.status, (await r.json()).error);
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'revoke failed';
  } finally {
    revokeTarget.value = null;
  }
}

function ago(iso: string | null) {
  if (!iso) return 'never';
  const d = new Date(iso);
  const s = Math.floor((Date.now() - d.getTime()) / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}
</script>

<template>
  <Shell>
    <div class="px-4 md:px-7 py-5 md:py-7">
      <h1 class="text-[20px] md:text-[22px] tracking-tight mb-1.5">api keys</h1>
      <p class="text-[12px] text-text-muted leading-relaxed mb-5 md:mb-6 max-w-lg">
        use these to publish from the terminal or from an agent. each key is shown
        once at creation — store it somewhere safe.
      </p>

      <div v-if="minted" class="bg-accent/5 border border-accent/30 rounded-sm px-3.5 py-3 mb-5 md:mb-6 text-[12px] text-text-dim leading-relaxed">
        <div class="text-accent mb-2">new key created. copy it now — you won't see it again:</div>
        <div class="flex items-center justify-between gap-2 bg-bg-deep border border-border rounded-sm px-3 py-2">
          <code class="text-[11px] md:text-[12px] text-text break-all min-w-0">{{ minted.token }}</code>
          <button class="text-[11px] text-accent shrink-0 hover:underline" @click="copyToken">copy</button>
        </div>
        <button class="text-[11px] text-text-muted hover:text-text mt-2" @click="minted = null">dismiss</button>
      </div>

      <div class="flex items-center justify-between mb-3">
        <div class="text-[11px] tracking-widest uppercase text-text-muted">active keys</div>
        <button
          v-if="!showForm"
          class="bg-accent text-bg-deep font-semibold px-3 py-1.5 text-[12px] rounded-sm hover:opacity-90"
          @click="showForm = true"
        >+ generate key</button>
      </div>

      <form v-if="showForm" class="bg-bg-deep border border-border rounded-sm px-4 py-4 mb-6 space-y-3" @submit.prevent="create">
        <label class="block">
          <span class="text-[11px] text-text-muted">label</span>
          <input v-model="newName" required maxlength="80" placeholder="laptop · zsh"
            class="mt-1.5 w-full bg-bg border border-border rounded-sm px-3 py-2 text-[13px] text-text focus:outline-none focus:border-accent" />
        </label>
        <div>
          <span class="text-[11px] text-text-muted">scopes</span>
          <div class="flex gap-5 mt-1.5">
            <label v-for="s in (['publish', 'read', 'delete'] as const)" :key="s" class="flex items-center gap-2 text-[13px]">
              <input type="checkbox" v-model="newScopes[s]" class="accent-[var(--color-accent)]" />
              <span>{{ s }}</span>
            </label>
          </div>
        </div>
        <div class="flex justify-between items-center pt-1">
          <button type="button" class="text-[12px] text-text-muted hover:text-text" @click="showForm = false">cancel</button>
          <button type="submit" class="bg-accent text-bg-deep font-semibold px-3 py-1.5 text-[12px] rounded-sm hover:opacity-90">create</button>
        </div>
      </form>

      <div v-if="error" class="text-[12px] text-danger mb-3">{{ error }}</div>
      <div v-if="loading && items.length === 0" class="text-[12px] text-text-muted">loading…</div>
      <div v-if="!loading && items.length === 0 && !minted" class="text-[12px] text-text-muted">no keys yet.</div>

      <ul class="space-y-2.5">
        <li v-for="k in items" :key="k.id" class="bg-bg-deep border border-border rounded-sm px-3.5 md:px-4 py-3 md:py-3.5">
          <div class="flex flex-wrap justify-between items-center gap-2 mb-2">
            <div class="text-[13px] text-text flex items-center gap-2 min-w-0">
              <span class="truncate">{{ k.name }}</span>
              <span v-if="k.revoked_at" class="text-[10px] uppercase tracking-widest text-danger">revoked</span>
            </div>
            <div v-if="!k.revoked_at" class="flex gap-1.5 shrink-0">
              <button
                class="text-text-muted border border-border-strong rounded-sm px-2.5 py-1 text-[12px] hover:text-text"
                @click="rotate"
              >rotate</button>
              <button
                class="text-danger border border-danger-border rounded-sm px-2.5 py-1 text-[12px] hover:bg-danger/10"
                @click="askRevoke(k)"
              >revoke</button>
            </div>
          </div>
          <div class="flex flex-wrap items-center gap-x-2 md:gap-x-3 gap-y-1 text-[10px] md:text-[11px] text-text-muted">
            <code class="text-text font-mono break-all">pds_live_{{ k.prefix }}··········</code>
            <span class="hidden md:inline">·</span>
            <span>{{ k.scopes.join(' · ') }}</span>
            <span class="hidden md:inline">·</span>
            <span>created {{ ago(k.created_at) }}</span>
            <span class="hidden md:inline">·</span>
            <span>used {{ ago(k.last_used_at) }}</span>
          </div>
        </li>
      </ul>
    </div>

    <Modal
      v-model:open="showRevoke"
      title="revoke this key?"
      danger
      @confirm="confirmRevoke"
    >
      any client using <code class="text-text">{{ revokeTarget ? `pds_live_${revokeTarget.prefix}` : '' }}</code>
      starts getting 401. this can't be undone.
      <template #actions>
        <button
          type="button"
          class="text-text-muted hover:text-text px-3 py-1.5 text-[12px]"
          @click="showRevoke = false"
        >cancel</button>
        <button
          type="button"
          class="bg-danger/10 text-danger border border-danger-border rounded-sm px-3 py-1.5 text-[12px] hover:bg-danger/20"
          @click="confirmRevoke"
        >revoke</button>
      </template>
    </Modal>
  </Shell>
</template>
