import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as apiClient from '../api';
import type { SetupStatus, UserPublic } from '../api/types';

export const useAuthStore = defineStore('auth', () => {
  const user = ref<UserPublic | null>(null);
  const setup = ref<SetupStatus | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastFetchAt = ref<number | null>(null);

  const needsSetup = computed(() => setup.value?.needs_setup === true);
  const isAdmin = computed(() => user.value?.role === 'admin');
  const isApproved = computed(() => user.value?.status === 'approved');

  async function refreshSetup() {
    try {
      setup.value = await apiClient.fetchSetupStatus();
    } catch (e) {
      setup.value = {
        needs_setup: false,
        version: 'unknown',
        checks: [
          {
            id: 'api',
            status: 'err',
            detail: e instanceof Error ? e.message : 'api unreachable',
          },
        ],
      };
    }
  }

  async function refreshMe() {
    try {
      user.value = await apiClient.me();
    } catch (e) {
      user.value = null;
    }
  }

  async function boot() {
    loading.value = true;
    error.value = null;
    await Promise.all([refreshSetup(), refreshMe()]);
    lastFetchAt.value = Date.now();
    loading.value = false;
  }

  async function login(input: { username: string; password: string }) {
    const { user: u } = await apiClient.login(input);
    user.value = u;
  }

  async function register(input: {
    username: string;
    email?: string;
    password: string;
    reason: string;
  }) {
    const { user: u } = await apiClient.register(input);
    user.value = u;
  }

  async function logout() {
    await apiClient.logout();
    user.value = null;
  }

  async function setupAdmin(input: {
    username: string;
    email?: string;
    password: string;
  }) {
    const { user: u } = await apiClient.createFirstAdmin(input);
    user.value = u;
    if (setup.value) {
      setup.value = { ...setup.value, needs_setup: false };
    }
  }

  return {
    user,
    setup,
    loading,
    error,
    lastFetchAt,
    needsSetup,
    isAdmin,
    isApproved,
    boot,
    refreshMe,
    refreshSetup,
    login,
    register,
    logout,
    setupAdmin,
  };
});
