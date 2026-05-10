<script setup lang="ts">
import { onMounted, ref } from 'vue';
import * as api from '../api';
import type { AdminUserView, UserStatus } from '../api/types';
import Shell from '../components/Shell.vue';
import { HttpError } from '../api';

type Tab = 'pending' | 'all';
const tab = ref<Tab>('pending');
const users = ref<AdminUserView[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function refresh() {
  loading.value = true;
  error.value = null;
  try {
    const filter: UserStatus | undefined = tab.value === 'pending' ? 'pending' : undefined;
    const list = await api.listUsers(filter);
    users.value = list.items;
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'failed to load';
  } finally {
    loading.value = false;
  }
}

onMounted(refresh);

async function act(verb: 'approve' | 'reject' | 'suspend' | 'restore' | 'promote' | 'demote', id: string) {
  try {
    if (verb === 'approve') await api.approveUser(id);
    else if (verb === 'reject') await api.rejectUser(id);
    else if (verb === 'suspend') await api.suspendUser(id);
    else if (verb === 'restore') await api.restoreUser(id);
    else if (verb === 'promote') await api.promoteUser(id);
    else if (verb === 'demote') await api.demoteUser(id);
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'action failed';
  }
}

function statusBadge(s: UserStatus): string {
  switch (s) {
    case 'pending': return 'text-yellow-400 border-yellow-400/50';
    case 'approved': return 'text-accent border-accent/50';
    case 'rejected': return 'text-rose-400 border-rose-400/50';
    case 'suspended': return 'text-text-muted border-border-strong';
  }
}
</script>

<template>
  <Shell>
    <div class="max-w-5xl mx-auto px-6 py-10">
      <div class="text-[11px] tracking-widest uppercase text-accent mb-2">paste · admin</div>
      <h1 class="text-lg font-medium mb-6">Users.</h1>

      <div class="flex gap-4 text-sm border-b border-border-strong mb-4">
        <button
          v-for="t in (['pending', 'all'] as const)"
          :key="t"
          @click="tab = t; refresh()"
          :class="[
            'px-1 py-2 -mb-px border-b-2',
            tab === t ? 'border-accent text-text' : 'border-transparent text-text-muted'
          ]"
        >{{ t }}</button>
        <button class="ml-auto text-xs text-text-muted hover:text-text" @click="refresh">refresh</button>
      </div>

      <div v-if="error" class="text-sm text-rose-400 mb-4">{{ error }}</div>
      <div v-if="loading" class="text-sm text-text-muted">loading…</div>

      <div v-if="!loading && users.length === 0" class="text-sm text-text-muted">No users to show.</div>

      <ul class="divide-y divide-border-strong">
        <li v-for="u in users" :key="u.id" class="py-4 grid grid-cols-[1.5fr_2fr_auto] gap-4 items-start">
          <div>
            <div class="font-medium">{{ u.username }}</div>
            <div class="text-xs text-text-muted">{{ u.email ?? '—' }} · {{ u.registration_ip ?? '—' }}</div>
            <span :class="['inline-block mt-2 px-2 py-0.5 text-[10px] uppercase tracking-widest border', statusBadge(u.status)]">{{ u.status }}</span>
            <span v-if="u.role === 'admin'" class="ml-2 text-[10px] uppercase tracking-widest text-accent">admin</span>
          </div>
          <div class="text-xs text-text-muted whitespace-pre-line max-w-md">{{ u.reason ?? '—' }}</div>
          <div class="flex flex-col items-end gap-1 text-xs">
            <template v-if="u.status === 'pending'">
              <button class="text-accent hover:underline" @click="act('approve', u.id)">approve</button>
              <button class="text-rose-400 hover:underline" @click="act('reject', u.id)">reject</button>
            </template>
            <template v-else-if="u.status === 'approved'">
              <button class="text-text-muted hover:text-text" @click="act('suspend', u.id)">suspend</button>
              <button v-if="u.role === 'user'" class="text-accent hover:underline" @click="act('promote', u.id)">promote</button>
              <button v-else class="text-text-muted hover:text-text" @click="act('demote', u.id)">demote</button>
            </template>
            <template v-else-if="u.status === 'suspended'">
              <button class="text-accent hover:underline" @click="act('restore', u.id)">restore</button>
            </template>
            <template v-else-if="u.status === 'rejected'">
              <button class="text-accent hover:underline" @click="act('restore', u.id)">restore</button>
            </template>
          </div>
        </li>
      </ul>
    </div>
  </Shell>
</template>
