<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { Snippet } from '../api';
import Shell from '../components/Shell.vue';
import SnippetPalette from '../components/SnippetPalette.vue';
import { LIFETIME_SECONDS, type LifetimeKey } from '../lib/lifetime';
import { useAuthStore } from '../stores/auth';
import { useToastStore } from '../stores/toast';
import { HttpError } from '../api';
import type { Visibility } from '../api/types';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const snippet = ref<Snippet | null>(null);
const error = ref<string | null>(null);
const toast = useToastStore();
const savingSettings = ref(false);

async function commitPolicy(patch: {
  visibility?: Visibility;
  lifetimeKey?: LifetimeKey;
  burnAfterRead?: boolean;
}) {
  if (!snippet.value) return;
  savingSettings.value = true;
  try {
    const apiPatch: {
      visibility?: Visibility;
      lifetime_seconds?: number | null;
      burn_after_read?: boolean;
    } = {};
    if (patch.visibility !== undefined) apiPatch.visibility = patch.visibility;
    if (patch.lifetimeKey !== undefined) apiPatch.lifetime_seconds = LIFETIME_SECONDS[patch.lifetimeKey];
    if (patch.burnAfterRead !== undefined) apiPatch.burn_after_read = patch.burnAfterRead;
    const updated = await api.updateSnippetSettings(snippet.value.slug, apiPatch);
    snippet.value = updated;
    toast.success('settings updated');
  } catch (e) {
    toast.error(e instanceof HttpError ? e.error.message : 'update failed');
  } finally {
    savingSettings.value = false;
  }
}

// The page is the user's html, full bleed, inside a 3px amber frame the
// embedded document cannot paint over — that frame is the "this is
// sandboxed user content" signal that used to be a banner. All app chrome
// (meta, policy, actions, nav) lives in the floating SnippetPalette.
//
// We load via `src=/h/:slug/raw` rather than splicing srcdoc on the client:
// the SPA's own CSP is `script-src 'self'`, and srcdoc iframes inherit it,
// which silently blocks all inline scripts inside the preview. Loading a real
// document lets the response's own CSP (`sandbox allow-scripts allow-popups`)
// take effect. The iframe `sandbox` attribute (no allow-same-origin) still
// gives the preview a null origin, so user code cannot read app cookies /
// storage.

onMounted(load);

async function load() {
  error.value = null;
  try {
    snippet.value = await api.getSnippet(route.params.slug as string);
    if (snippet.value.type !== 'html') {
      const prefix = snippet.value.type === 'markdown' ? '/m/' : '/c/';
      router.replace(`${prefix}${snippet.value.slug}`);
    }
  } catch (e) {
    if (e instanceof HttpError && e.status === 401) {
      router.replace({ name: 'signin', query: { next: route.fullPath } });
      return;
    }
    error.value = e instanceof HttpError ? e.error.message : 'load failed';
    snippet.value = null;
  }
}

async function remove() {
  if (!snippet.value) return;
  try {
    await api.deleteSnippet(snippet.value.slug);
    router.replace('/dashboard');
  } catch (e) {
    toast.error(e instanceof HttpError ? e.error.message : 'delete failed');
  }
}
const canEdit = (s: Snippet | null) => !!s && auth.user?.username === s.owner.username;
</script>

<template>
  <Shell v-if="error">
    <div class="px-4 md:px-7 py-5 md:py-6 text-[12px] text-danger">{{ error }}</div>
  </Shell>
  <div
    v-else-if="snippet && snippet.type === 'html'"
    class="fixed inset-0 p-[3px] bg-warn/55"
  >
    <iframe
      :src="snippet.raw_url"
      sandbox="allow-scripts allow-popups"
      referrerpolicy="no-referrer"
      :title="`${snippet.name ?? snippet.slug} · user html`"
      class="block w-full h-full bg-white border-0"
    />
    <SnippetPalette
      :snippet="snippet"
      :can-edit="canEdit(snippet)"
      :pending="savingSettings"
      @commit="commitPolicy"
      @remove="remove"
    />
  </div>
</template>
