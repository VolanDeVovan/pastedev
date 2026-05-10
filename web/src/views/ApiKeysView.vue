<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { config } from '../config';
import Shell from '../components/Shell.vue';
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
  const scopes: Scope[] = (Object.keys(newScopes.value) as Scope[]).filter(
    (s) => newScopes.value[s],
  );
  try {
    const r = await fetch(`${config.apiBaseUrl}/api/v1/keys`, {
      method: 'POST',
      credentials: 'include',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ name: newName.value.trim(), scopes }),
    });
    if (!r.ok) throw new HttpError(r.status, (await r.json()).error);
    const data = (await r.json()) as KeyMinted;
    minted.value = data;
    newName.value = '';
    newScopes.value = { publish: true, read: false, delete: false };
    showForm.value = false;
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'create failed';
  }
}

async function copyToken() {
  if (!minted.value) return;
  await navigator.clipboard.writeText(minted.value.token);
}

async function revoke(id: string) {
  if (!confirm('revoke this key? any client using it will start getting 401.')) return;
  try {
    const r = await fetch(`${config.apiBaseUrl}/api/v1/keys/${id}`, {
      method: 'DELETE',
      credentials: 'include',
    });
    if (!r.ok) throw new HttpError(r.status, (await r.json()).error);
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'revoke failed';
  }
}

function fmt(d: string | null) {
  return d ? new Date(d).toLocaleString() : '—';
}
</script>

<template>
  <Shell>
    <div class="max-w-4xl mx-auto px-6 py-10">
      <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · keys</div>
      <h1 class="text-lg font-medium mb-6">API keys.</h1>

      <div v-if="minted" class="border border-amber-400/40 bg-amber-400/5 p-4 mb-6">
        <div class="text-[11px] uppercase tracking-widest text-amber-300 mb-1">one-time secret</div>
        <p class="text-xs text-text-muted mb-2">
          This token won't be shown again. Store it now.
        </p>
        <div class="flex items-center gap-2">
          <code class="flex-1 bg-bg-deep border border-border-strong px-3 py-2 text-xs break-all">{{ minted.token }}</code>
          <button class="text-xs border border-accent text-accent px-3 py-2 hover:bg-accent hover:text-bg-deep" @click="copyToken">copy</button>
          <button class="text-xs text-text-muted hover:text-text" @click="minted = null">dismiss</button>
        </div>
      </div>

      <div class="flex items-center justify-between mb-3">
        <p class="text-xs text-text-muted">{{ items.length }} key{{ items.length === 1 ? '' : 's' }}</p>
        <button v-if="!showForm" class="text-xs border border-accent text-accent px-3 py-1.5 hover:bg-accent hover:text-bg-deep" @click="showForm = true">+ new key</button>
      </div>

      <form v-if="showForm" class="border border-border-strong border-l-[3px] border-l-accent p-4 mb-6 space-y-3" @submit.prevent="create">
        <label class="block">
          <span class="text-[11px] uppercase tracking-widest text-text-muted">label</span>
          <input v-model="newName" required maxlength="80" placeholder="laptop · zsh"
            class="mt-1.5 w-full bg-panel border border-border-strong px-3 py-2 text-sm focus:outline-none focus:border-accent" />
        </label>
        <div>
          <span class="text-[11px] uppercase tracking-widest text-text-muted">scopes</span>
          <div class="flex gap-4 mt-1.5">
            <label v-for="s in (['publish', 'read', 'delete'] as const)" :key="s" class="flex items-center gap-2 text-sm">
              <input type="checkbox" v-model="newScopes[s]" class="accent-accent" />
              <span>{{ s }}</span>
            </label>
          </div>
        </div>
        <div class="flex justify-between items-center pt-2">
          <button type="button" class="text-xs text-text-muted hover:text-text" @click="showForm = false">cancel</button>
          <button type="submit" class="text-xs border border-accent text-accent px-3 py-1.5 hover:bg-accent hover:text-bg-deep">create</button>
        </div>
      </form>

      <div v-if="error" class="text-sm text-rose-400 mb-3">{{ error }}</div>
      <ul class="divide-y divide-border-strong">
        <li v-for="k in items" :key="k.id" class="py-3 grid grid-cols-[1fr_auto_auto_auto] gap-4 items-center">
          <div>
            <div class="text-sm font-medium">{{ k.name }} <span v-if="k.revoked_at" class="ml-2 text-[10px] uppercase tracking-widest text-rose-400">revoked</span></div>
            <div class="text-xs text-text-muted">
              <code>pds_live_{{ k.prefix }}_•••</code>
              · {{ k.scopes.join(' / ') }}
              · last used {{ fmt(k.last_used_at) }}
              · created {{ fmt(k.created_at) }}
            </div>
          </div>
          <button v-if="!k.revoked_at" class="text-xs text-rose-400 hover:underline" @click="revoke(k.id)">revoke</button>
        </li>
      </ul>
    </div>
  </Shell>
</template>
