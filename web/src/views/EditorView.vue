<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { SnippetType } from '../api/types';
import Shell from '../components/Shell.vue';
import { useHighlight } from '../composables/useHighlight';
import { renderMarkdown } from '../lib/markdown';
import { useAuthStore } from '../stores/auth';
import { useToastStore } from '../stores/toast';
import { HttpError } from '../api';

const route = useRoute();
const router = useRouter();
const auth = useAuthStore();
const toast = useToastStore();

const kind = ref<SnippetType>('code');
const body = ref('');
const name = ref('');
const error = ref<string | null>(null);
const submitting = ref(false);
const editingSlug = ref<string | null>(null);

const showSize = computed(() => new Blob([body.value]).size);
const isOverLimit = computed(() => showSize.value > 1_048_576);

// The editor itself doesn't render highlighted code — the worker still runs to
// report the detected language in the bottom hint and to test the large-file
// fast path while typing.
const { language: detectedLang, truncated: hlTruncated, highlight } = useHighlight();
const markdownHtml = ref('');
let mdDebounceHandle: number | null = null;

// HTML live preview srcdoc — debounced so we don't blow away the iframe on
// every keystroke. The iframe is sandboxed (allow-scripts + allow-popups),
// identical to the published view, so the editor preview is a faithful
// representation of what the visitor will see.
const htmlSrcdoc = ref('');
let htmlDebounceHandle: number | null = null;

watch([body, kind], ([newBody, newKind]) => {
  if (newKind === 'code') {
    highlight(newBody);
    return;
  }
  if (newKind === 'markdown') {
    if (mdDebounceHandle !== null) clearTimeout(mdDebounceHandle);
    mdDebounceHandle = window.setTimeout(() => {
      markdownHtml.value = renderMarkdown(newBody);
    }, 120);
    return;
  }
  if (newKind === 'html') {
    if (htmlDebounceHandle !== null) clearTimeout(htmlDebounceHandle);
    htmlDebounceHandle = window.setTimeout(() => {
      htmlSrcdoc.value = newBody;
    }, 250);
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
      router.replace(new URL(updated.url).pathname);
    } else {
      const created = await api.createSnippet({
        type: kind.value,
        name: name.value.trim() || undefined,
        body: body.value,
      });
      router.replace(new URL(created.url).pathname);
    }
  } catch (e) {
    if (e instanceof HttpError && e.error.code === 'forbidden') await auth.refreshMe();
    error.value = e instanceof HttpError ? e.error.message : 'publish failed';
    toast.error(error.value ?? 'publish failed');
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

// markdown + html get a split preview. Code mode stays single column —
// syntax highlighting kicks in on the published /c/:slug view, so the editor
// doesn't need a duplicated preview pane that just adds scroll noise.
const splitPreview = computed(() => kind.value === 'markdown' || kind.value === 'html');

const placeholder = computed(() => {
  switch (kind.value) {
    case 'code': return '// paste code, text, or any content here…';
    case 'markdown': return '# title\n\nstart writing markdown…';
    case 'html': return '<!doctype html>\n<html>\n  <body>\n    <h1>hello</h1>\n  </body>\n</html>';
  }
});
</script>

<template>
  <Shell>
    <!-- Editor takes the full viewport minus the TopBar (52px). Three rows:
         toolbar / body / statusline. The body grows; nothing overlaps. -->
    <div class="flex flex-col h-[calc(100vh-52px)]">
      <!-- toolbar -->
      <div class="flex items-center justify-between px-7 py-2.5 border-b border-border text-[12px] shrink-0">
        <div class="flex gap-1.5">
          <button
            v-for="t in (['code', 'markdown', 'html'] as const)"
            :key="t"
            :class="[
              'px-3.5 py-1.5 rounded-sm border',
              kind === t ? 'bg-border text-text border-border-strong' : 'text-text-muted border-transparent hover:text-text',
            ]"
            @click="kind = t"
          >{{ t }}</button>
        </div>
        <div class="flex gap-3 items-center">
          <input
            v-model="name"
            placeholder="filename · optional"
            class="bg-bg-deep border border-border rounded-sm px-2.5 py-1 text-[12px] text-text focus:outline-none focus:border-accent w-56"
          />
          <!-- Decorative read-only dropdowns. v1 fixes expiry and visibility per
               the spec, but the design slot is preserved so we don't have to
               re-balance the toolbar later when these become real controls. -->
          <span
            class="bg-bg-deep border border-border rounded-sm px-2.5 py-1 text-[11px] text-text-dim opacity-70"
            style="cursor: default"
            title="snippets don't expire in v1"
          >expires: never ▾</span>
          <span
            class="bg-bg-deep border border-border rounded-sm px-2.5 py-1 text-[11px] text-text-dim opacity-70"
            style="cursor: default"
            title="all snippets are public in v1"
          >visibility: public ▾</span>
          <span :class="['text-[11px]', isOverLimit ? 'text-danger' : 'text-text-muted']">{{ showSize.toLocaleString() }} b</span>
          <span class="w-px h-4 bg-border-strong" />
          <button
            :disabled="submitting || isOverLimit"
            class="bg-accent text-bg-deep font-semibold px-3.5 py-1 text-[12px] rounded-sm hover:opacity-90 disabled:opacity-30"
            @click="submit"
          >{{ submitting ? '…' : editingSlug ? 'save ⌘↵' : 'publish ⌘↵' }}</button>
        </div>
      </div>

      <!-- body -->
      <div :class="['grid flex-1 min-h-0', splitPreview ? 'grid-cols-2' : 'grid-cols-1']">
        <div :class="['overflow-auto px-7 py-5', splitPreview ? 'border-r border-border' : '']">
          <textarea
            v-model="body"
            spellcheck="false"
            wrap="soft"
            class="w-full h-full bg-transparent text-[13px] text-text font-mono leading-relaxed resize-none focus:outline-none whitespace-pre-wrap break-words"
            :placeholder="placeholder"
            @keydown="handleKeydown"
          />
        </div>

        <div v-if="kind === 'markdown'" class="overflow-auto px-9 py-7 bg-bg-deep">
          <div class="text-[10px] tracking-widest uppercase text-text-muted mb-3.5">preview</div>
          <article class="md-preview" v-html="markdownHtml || '<p style=\'color:var(--color-text-faint)\'>start writing — preview shows here.</p>'" />
        </div>

        <div v-if="kind === 'html'" class="bg-bg-deep flex flex-col min-h-0">
          <div class="flex items-center justify-between px-5 pt-5 pb-2.5 shrink-0">
            <div class="text-[10px] tracking-widest uppercase text-text-muted">live preview · sandboxed</div>
            <div class="text-[10px] text-text-faint">scripts allowed · app-origin blocked</div>
          </div>
          <!-- Empty-state placeholder. We render the iframe only once the user
               has typed something so the default white "untitled" page from a
               blank document doesn't flash in. The body, once present, is
               passed through verbatim — no style injection, what the visitor
               sees is exactly what you wrote. -->
          <div v-if="!htmlSrcdoc.trim()" class="flex-1 mx-5 mb-5 border border-border rounded-sm border-dashed flex items-center justify-center text-center px-6">
            <div>
              <div class="text-[11px] tracking-widest uppercase text-text-faint mb-2">empty draft</div>
              <div class="text-[12px] text-text-muted max-w-xs leading-relaxed">
                start typing on the left — the preview renders in a sandboxed
                iframe with the page's own styles, untouched.
              </div>
            </div>
          </div>
          <iframe
            v-else
            :srcdoc="htmlSrcdoc"
            sandbox="allow-scripts allow-popups"
            referrerpolicy="no-referrer"
            title="html live preview"
            class="flex-1 mx-5 mb-5 bg-white border border-border rounded-sm"
          />
        </div>
      </div>

      <!-- statusline (its own row, no overlap with the textarea) -->
      <div class="flex items-center justify-between px-7 py-2 border-t border-border text-[11px] shrink-0">
        <div class="flex gap-3 text-text-faint">
          <span>⌘+enter to publish</span>
          <span v-if="kind === 'code' && detectedLang && !hlTruncated" class="text-text-muted">· detected · {{ detectedLang }}</span>
          <span v-if="hlTruncated" class="text-warn">· syntax highlighting off · large file</span>
        </div>
        <div v-if="error" class="text-danger">{{ error }}</div>
      </div>
    </div>
  </Shell>
</template>
