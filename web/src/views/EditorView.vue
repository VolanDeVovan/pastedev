<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { SnippetType } from '../api/types';
import Shell from '../components/Shell.vue';
import { useHighlight } from '../composables/useHighlight';
import { renderMarkdown } from '../lib/markdown';
import { useAuthStore } from '../stores/auth';
import { HttpError } from '../api';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();

const kind = ref<SnippetType>('code');
const body = ref('');
const name = ref('');
const error = ref<string | null>(null);
const submitting = ref(false);
const editingSlug = ref<string | null>(null);
const showSize = computed(() => new Blob([body.value]).size);
const isOverLimit = computed(() => showSize.value > 1_048_576);

const { html: highlightedHtml, language: detectedLang, highlight } = useHighlight();
const markdownHtml = ref('');
let mdDebounceHandle: number | null = null;

watch([body, kind], ([newBody, newKind]) => {
  if (newKind === 'code') {
    highlight(newBody);
  } else if (newKind === 'markdown') {
    if (mdDebounceHandle !== null) clearTimeout(mdDebounceHandle);
    mdDebounceHandle = window.setTimeout(() => {
      markdownHtml.value = renderMarkdown(newBody);
    }, 120);
  }
});

onMounted(async () => {
  const edit = route.query.edit;
  if (typeof edit === 'string') {
    try {
      const s = await api.getSnippet(edit);
      editingSlug.value = s.slug;
      kind.value = s.type;
      body.value = s.body;
      name.value = s.name ?? '';
    } catch (e) {
      error.value = e instanceof HttpError ? e.error.message : 'failed to load snippet';
    }
  }
});

async function submit() {
  error.value = null;
  if (!body.value) {
    error.value = 'body is empty';
    return;
  }
  if (isOverLimit.value) {
    error.value = `body exceeds 1 MB (${showSize.value.toLocaleString()} bytes)`;
    return;
  }
  submitting.value = true;
  try {
    if (editingSlug.value) {
      const updated = await api.patchSnippet(editingSlug.value, {
        body: body.value,
        name: name.value.trim() || null,
      });
      router.replace(updated.url.replace(location.origin, ''));
    } else {
      const created = await api.createSnippet({
        type: kind.value,
        name: name.value.trim() || undefined,
        body: body.value,
      });
      router.replace(created.url.replace(location.origin, ''));
    }
  } catch (e) {
    if (e instanceof HttpError && e.error.code === 'forbidden') {
      await auth.refreshMe();
    }
    error.value = e instanceof HttpError ? e.error.message : 'publish failed';
  } finally {
    submitting.value = false;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
    e.preventDefault();
    submit();
  }
}
</script>

<template>
  <Shell>
    <div class="max-w-5xl mx-auto px-6 py-8">
      <div class="text-[11px] tracking-widest uppercase text-accent mb-2">
        paste · {{ editingSlug ? 'edit' : 'new' }}
      </div>
      <h1 class="text-lg font-medium mb-6">
        {{ editingSlug ? `Editing ${editingSlug}` : 'Publish a snippet.' }}
      </h1>

      <div class="flex items-center gap-4 mb-3">
        <div class="flex border border-border-strong">
          <button
            v-for="t in (['code', 'markdown', 'html'] as const)"
            :key="t"
            :class="[
              'px-3 py-1.5 text-xs uppercase tracking-widest',
              kind === t ? 'bg-accent text-bg-deep' : 'text-text-muted hover:text-text',
            ]"
            @click="kind = t"
          >{{ t }}</button>
        </div>
        <input
          v-model="name"
          placeholder="filename · optional"
          class="flex-1 bg-panel border border-border-strong px-3 py-1.5 text-sm focus:outline-none focus:border-accent"
        />
        <span class="text-xs text-text-muted">{{ showSize.toLocaleString() }} b</span>
        <button
          :disabled="submitting || isOverLimit"
          class="text-xs border border-accent text-accent px-4 py-1.5 hover:bg-accent hover:text-bg-deep transition-colors disabled:opacity-30"
          @click="submit"
        >{{ submitting ? '…' : editingSlug ? 'save' : 'publish' }}</button>
      </div>

      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div>
          <div class="text-[11px] uppercase tracking-widest text-text-muted mb-1">source</div>
          <textarea
            v-model="body"
            spellcheck="false"
            class="w-full h-[60vh] bg-panel border border-border-strong p-3 text-sm font-mono focus:outline-none focus:border-accent resize-none"
            :placeholder="kind === 'code' ? '// paste code here' : '# markdown title\n\nsome **bold** text'"
            @keydown="handleKeydown"
          />
        </div>
        <div>
          <div class="text-[11px] uppercase tracking-widest text-text-muted mb-1 flex justify-between">
            <span>preview</span>
            <span v-if="kind === 'code' && detectedLang">detected · {{ detectedLang }}</span>
          </div>
          <div class="h-[60vh] overflow-auto bg-panel border border-border-strong p-3 text-sm">
            <pre v-if="kind === 'code'"><code class="hljs" v-html="highlightedHtml || body" /></pre>
            <div v-else-if="kind === 'markdown'" class="prose prose-invert prose-sm max-w-none" v-html="markdownHtml" />
            <div v-else class="text-text-muted text-xs leading-relaxed">
              <p class="font-medium text-text mb-2">html · sandboxed</p>
              <p>Publish the snippet to render it. The view route serves the body with
                <code>Content-Security-Policy: sandbox allow-scripts allow-popups</code> and the
                SPA renders it inside <code>&lt;iframe sandbox="allow-scripts allow-popups"&gt;</code>.</p>
              <p class="mt-2">Cookies, credentialed fetches to the app origin, service workers, and
                cross-frame navigation are all blocked.</p>
            </div>
          </div>
        </div>
      </div>

      <div v-if="error" class="mt-4 text-sm text-rose-400">{{ error }}</div>
      <p class="text-xs text-text-muted mt-3">
        Press <kbd class="px-1 border border-border-strong">⌘</kbd> +
        <kbd class="px-1 border border-border-strong">↵</kbd> to publish.
      </p>
    </div>
  </Shell>
</template>
