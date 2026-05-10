<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
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

function statusColor(s: UserStatus): string {
  switch (s) {
    case 'pending': return 'text-warn';
    case 'approved': return 'text-accent';
    case 'rejected': return 'text-danger';
    case 'suspended': return 'text-text-muted';
  }
}

const oldest = computed(() => {
  if (users.value.length === 0) return null;
  const t = users.value.reduce((min, u) => Math.min(min, new Date(u.created_at).getTime()), Date.now());
  const s = Math.floor((Date.now() - t) / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
});
</script>

<template>
  <Shell>
    <div class="max-w-5xl mx-auto px-7 py-8">
      <h1 class="text-[22px] tracking-tight mb-1.5">admin · {{ tab }} users</h1>
      <p class="text-[12px] text-text-muted mb-6">
        {{ users.length }} {{ tab === 'pending' ? 'pending request' : 'user' }}{{ users.length === 1 ? '' : 's' }}
        <template v-if="oldest"> · oldest {{ oldest }}</template>
      </p>

      <div class="flex gap-5 text-[12px] border-b border-border mb-3">
        <button
          v-for="t in (['pending', 'all'] as const)"
          :key="t"
          @click="tab = t; refresh()"
          :class="[
            'px-1 pb-2 -mb-px border-b',
            tab === t ? 'border-accent text-text' : 'border-transparent text-text-muted hover:text-text'
          ]"
        >{{ t }}</button>
        <button class="ml-auto pb-2 text-[11px] text-text-muted hover:text-text" @click="refresh">refresh</button>
      </div>

      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="loading" class="text-[12px] text-text-muted py-4">loading…</div>
      <div v-if="!loading && users.length === 0" class="text-[12px] text-text-muted py-4">no users to show.</div>

      <ul class="divide-y divide-border">
        <li v-for="u in users" :key="u.id" class="py-4 grid grid-cols-[1.4fr_2fr_auto] gap-5 items-start">
          <div>
            <div class="text-[13px] text-text">{{ u.username }}</div>
            <div class="text-[11px] text-text-muted mt-0.5">{{ u.email ?? '—' }} · {{ u.registration_ip ?? '—' }}</div>
            <div class="mt-2 flex gap-3 text-[10px] tracking-widest uppercase">
              <span :class="statusColor(u.status)">{{ u.status }}</span>
              <span v-if="u.role === 'admin'" class="text-accent">admin</span>
            </div>
          </div>
          <div class="text-[12px] text-text-muted leading-relaxed whitespace-pre-line max-w-lg">{{ u.reason ?? '—' }}</div>
          <div class="flex flex-col items-end gap-1.5 text-[12px]">
            <template v-if="u.status === 'pending'">
              <button class="bg-accent text-bg-deep font-semibold px-3 py-1 text-[12px] rounded-sm hover:opacity-90" @click="act('approve', u.id)">approve</button>
              <button class="text-danger border border-danger-border rounded-sm px-2.5 py-1 hover:bg-danger/10" @click="act('reject', u.id)">reject</button>
            </template>
            <template v-else-if="u.status === 'approved'">
              <button class="text-text-muted border border-border-strong rounded-sm px-2.5 py-1 hover:text-text" @click="act('suspend', u.id)">suspend</button>
              <button v-if="u.role === 'user'" class="text-accent border border-accent/40 rounded-sm px-2.5 py-1 hover:bg-accent/10" @click="act('promote', u.id)">promote</button>
              <button v-else class="text-text-muted border border-border-strong rounded-sm px-2.5 py-1 hover:text-text" @click="act('demote', u.id)">demote</button>
            </template>
            <template v-else-if="u.status === 'suspended' || u.status === 'rejected'">
              <button class="text-accent border border-accent/40 rounded-sm px-2.5 py-1 hover:bg-accent/10" @click="act('restore', u.id)">restore</button>
            </template>
          </div>
        </li>
      </ul>
    </div>
  </Shell>
</template>
