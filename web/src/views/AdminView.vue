<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import * as api from '../api';
import type { AdminUserView, UserStatus } from '../api/types';
import Shell from '../components/Shell.vue';
import Modal from '../components/Modal.vue';
import { HttpError } from '../api';

type Tab = 'pending' | 'all';
const tab = ref<Tab>('pending');
const users = ref<AdminUserView[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

// Confirmation modals for the destructive/visible actions. Promote/demote/
// suspend/restore remain direct — less destructive, and the admin can always
// flip them back.
const showApprove = ref(false);
const showReject = ref(false);
const target = ref<AdminUserView | null>(null);

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

async function act(
  verb: 'suspend' | 'restore' | 'promote' | 'demote',
  id: string,
) {
  try {
    if (verb === 'suspend') await api.suspendUser(id);
    else if (verb === 'restore') await api.restoreUser(id);
    else if (verb === 'promote') await api.promoteUser(id);
    else if (verb === 'demote') await api.demoteUser(id);
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'action failed';
  }
}

function askApprove(u: AdminUserView) {
  target.value = u;
  showApprove.value = true;
}
function askReject(u: AdminUserView) {
  target.value = u;
  showReject.value = true;
}

async function confirmApprove() {
  if (!target.value) return;
  const id = target.value.id;
  showApprove.value = false;
  try {
    await api.approveUser(id);
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'approve failed';
  } finally {
    target.value = null;
  }
}

async function confirmReject() {
  if (!target.value) return;
  const id = target.value.id;
  showReject.value = false;
  try {
    await api.rejectUser(id);
    await refresh();
  } catch (e) {
    error.value = e instanceof HttpError ? e.error.message : 'reject failed';
  } finally {
    target.value = null;
  }
}

function ago(iso: string): string {
  const d = new Date(iso);
  const s = Math.floor((Date.now() - d.getTime()) / 1000);
  if (s < 60) return `${s}s ago`;
  if (s < 3600) return `${Math.floor(s / 60)}m ago`;
  if (s < 86400) return `${Math.floor(s / 3600)}h ago`;
  return `${Math.floor(s / 86400)}d ago`;
}

const oldestAgo = computed(() => {
  if (users.value.length === 0) return null;
  const t = users.value.reduce(
    (min, u) => Math.min(min, new Date(u.created_at).getTime()),
    Date.now(),
  );
  return ago(new Date(t).toISOString());
});

const pendingCount = computed(
  () => users.value.filter((u) => u.status === 'pending').length,
);

function statusColor(s: UserStatus): string {
  switch (s) {
    case 'pending': return 'text-warn';
    case 'approved': return 'text-accent';
    case 'rejected': return 'text-danger';
    case 'suspended': return 'text-text-muted';
  }
}

// Deterministic avatar colors derived from the username — same input always
// produces the same hue, so a user's circle is recognizable across rerenders.
const palette = [
  'bg-accent/20 text-accent',
  'bg-warn/20 text-warn',
  'bg-blue-400/20 text-blue-300',
  'bg-rose-400/20 text-rose-300',
  'bg-purple-400/20 text-purple-300',
  'bg-emerald-400/20 text-emerald-300',
];
function avatarClass(username: string): string {
  let h = 0;
  for (let i = 0; i < username.length; i++) h = (h * 31 + username.charCodeAt(i)) >>> 0;
  return palette[h % palette.length];
}
</script>

<template>
  <Shell>
    <div class="px-4 md:px-7 py-5 md:py-7">
      <h1 class="text-[20px] md:text-[22px] tracking-tight mb-1.5">admin · {{ tab }} users</h1>
      <p class="text-[12px] text-text-muted mb-4 md:mb-5">
        {{ users.length }} {{ tab === 'pending' ? 'pending request' : 'user' }}{{ users.length === 1 ? '' : 's' }}
        <template v-if="oldestAgo"> · oldest from {{ oldestAgo }}</template>
      </p>

      <div class="flex items-center gap-1.5 mb-1 text-[11px] -mx-1 px-1 overflow-x-auto">
        <button
          v-for="t in (['pending', 'all'] as const)"
          :key="t"
          @click="tab = t; refresh()"
          :class="[
            'px-3 md:px-3.5 py-1.5 md:py-2 rounded-sm border whitespace-nowrap shrink-0',
            tab === t
              ? 'bg-border text-text border-border-strong'
              : 'border-transparent text-text-muted hover:text-text',
          ]"
        >{{ t }}<template v-if="t === 'pending'"> · {{ pendingCount }}</template></button>
        <button class="ml-auto text-[11px] text-text-muted hover:text-text shrink-0" @click="refresh">refresh</button>
      </div>

      <div v-if="error" class="text-[12px] text-danger mb-4">{{ error }}</div>
      <div v-if="loading" class="text-[12px] text-text-muted py-4">loading…</div>
      <div v-if="!loading && users.length === 0" class="text-[12px] text-text-muted py-4">no users to show.</div>

      <ul>
        <!-- On desktop: avatar | reason | actions in three columns.
             On mobile: stack — avatar+meta row, reason below, actions at the
             bottom as a full-width button row (see mobile.jsx MAdmin). -->
        <li
          v-for="u in users"
          :key="u.id"
          class="flex flex-col md:grid md:grid-cols-[1.4fr_1fr_auto] gap-3 md:gap-6 py-4 border-b border-border md:items-start"
        >
          <div class="flex items-start gap-3">
            <div
              :class="[
                avatarClass(u.username),
                'w-9 h-9 rounded-full flex items-center justify-center text-[13px] font-semibold uppercase shrink-0',
              ]"
            >{{ u.username.charAt(0) }}</div>
            <div class="min-w-0 flex-1">
              <div class="text-[14px] text-text truncate">{{ u.username }}</div>
              <div class="text-[11px] text-text-muted mt-0.5 truncate">{{ u.email ?? '—' }}</div>
              <div class="text-[11px] text-text-muted mt-0.5">
                requested {{ ago(u.created_at) }}
                <template v-if="u.registration_ip"> · {{ u.registration_ip }}</template>
              </div>
              <div class="mt-1.5 flex gap-3 text-[10px] tracking-widest uppercase">
                <span :class="statusColor(u.status)">{{ u.status }}</span>
                <span v-if="u.role === 'admin'" class="text-accent">admin</span>
              </div>
            </div>
          </div>
          <div v-if="u.reason" class="text-[12px] text-text-dim leading-relaxed whitespace-pre-line md:block">{{ u.reason }}</div>
          <div v-else class="hidden md:block text-[12px] text-text-dim">—</div>
          <div class="flex md:flex-col md:items-end gap-1.5 text-[12px]">
            <template v-if="u.status === 'pending'">
              <button
                class="text-danger border border-danger-border rounded-sm px-2.5 py-1 hover:bg-danger/10 flex-1 md:flex-none order-2 md:order-1"
                @click="askReject(u)"
              >reject</button>
              <button
                class="bg-accent text-bg-deep font-semibold rounded-sm px-3 py-1 hover:opacity-90 flex-1 md:flex-none order-1 md:order-2"
                @click="askApprove(u)"
              >approve</button>
            </template>
            <template v-else-if="u.status === 'approved'">
              <button
                class="text-text-muted border border-border-strong rounded-sm px-2.5 py-1 hover:text-text flex-1 md:flex-none"
                @click="act('suspend', u.id)"
              >suspend</button>
              <button
                v-if="u.role === 'user'"
                class="text-accent border border-accent/40 rounded-sm px-2.5 py-1 hover:bg-accent/10 flex-1 md:flex-none"
                @click="act('promote', u.id)"
              >promote</button>
              <button
                v-else
                class="text-text-muted border border-border-strong rounded-sm px-2.5 py-1 hover:text-text flex-1 md:flex-none"
                @click="act('demote', u.id)"
              >demote</button>
            </template>
            <template v-else-if="u.status === 'suspended' || u.status === 'rejected'">
              <button
                class="text-accent border border-accent/40 rounded-sm px-2.5 py-1 hover:bg-accent/10 flex-1 md:flex-none"
                @click="act('restore', u.id)"
              >restore</button>
            </template>
          </div>
        </li>
      </ul>
    </div>

    <Modal v-model:open="showApprove" title="approve this user?" @confirm="confirmApprove">
      <div v-if="target">
        <div class="text-text">{{ target.username }}</div>
        <div class="text-text-muted text-[11px] mt-0.5">{{ target.email ?? '—' }}</div>
      </div>
      <template #actions>
        <button
          type="button"
          class="text-text-muted hover:text-text px-3 py-1.5 text-[12px]"
          @click="showApprove = false"
        >cancel</button>
        <button
          type="button"
          class="bg-accent text-bg-deep font-semibold rounded-sm px-3 py-1.5 text-[12px] hover:opacity-90"
          @click="confirmApprove"
        >approve</button>
      </template>
    </Modal>

    <Modal v-model:open="showReject" title="reject this user?" danger @confirm="confirmReject">
      <div v-if="target">
        <div class="text-text">{{ target.username }}</div>
        <div class="text-text-muted text-[11px] mt-0.5">{{ target.email ?? '—' }}</div>
      </div>
      <template #actions>
        <button
          type="button"
          class="text-text-muted hover:text-text px-3 py-1.5 text-[12px]"
          @click="showReject = false"
        >cancel</button>
        <button
          type="button"
          class="bg-danger/10 text-danger border border-danger-border rounded-sm px-3 py-1.5 text-[12px] hover:bg-danger/20"
          @click="confirmReject"
        >reject</button>
      </template>
    </Modal>
  </Shell>
</template>
