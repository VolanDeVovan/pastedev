<script setup lang="ts">
import { ref, computed, watch, onMounted } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import * as api from '../api';
import type { SnippetType, Visibility } from '../api/types';
import Shell from '../components/Shell.vue';
import PolicyBar from '../components/PolicyBar.vue';
import { LIFETIME_SECONDS, type LifetimeKey } from '../lib/lifetime';
import PublishOptionsModal from '../components/PublishOptionsModal.vue';
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

// Sharing-policy draft state.
//
// `visibility` and `burnAfterRead` are direct v-models on PolicyBar pills.
// Lifetime is a little subtler — PolicyBar renders its label from an
// `expiresAt` timestamp.
//
//   * Creation flow: the user picks a preset (`lifetimeKey`) from PolicyBar
//     or the publish modal. We synthesize the absolute expiry locally
//     (`now() + LIFETIME_SECONDS[key]`) just for the pill display. On
//     publish we send `lifetime_seconds`; the server re-stamps `expires_at`
//     against its own clock.
//
//   * Edit flow: the server's response to /settings carries the absolute
//     `expires_at` we should show next. We park it in `serverExpiresAt`
//     and PolicyBar reads from there.
const visibility = ref<Visibility>('public');
const burnAfterRead = ref(false);
const lifetimeKey = ref<LifetimeKey>('never');
const serverExpiresAt = ref<string | null>(null);

const displayExpiresAt = computed(() => {
  if (editingSlug.value) return serverExpiresAt.value;
  const seconds = LIFETIME_SECONDS[lifetimeKey.value];
  if (seconds == null) return null;
  return new Date(Date.now() + seconds * 1000).toISOString();
});

// Mobile-only publish modal — on phones the toolbar can't reasonably fit
// 3 dropdowns + a checkbox + a publish button + a filename input. Desktop
// keeps the inline controls; mobile uses this modal to expose the same
// state. The `showPublishOptions` ref is shared so both flows write into
// the same `visibility` / `lifetimeKey` / `burnAfterRead` refs above.
const showPublishOptions = ref(false);

// Decide whether the publish button should open the policy modal (mobile,
// creating) or submit immediately (desktop, or editing — both have the
// policy already visible/locked). 768px matches Tailwind's `md:` breakpoint.
function onPublishClick() {
  if (editingSlug.value) {
    submit();
    return;
  }
  const wantsModal = typeof window !== 'undefined'
    && !window.matchMedia('(min-width: 768px)').matches;
  if (wantsModal) {
    showPublishOptions.value = true;
  } else {
    submit();
  }
}

async function submitFromModal() {
  showPublishOptions.value = false;
  await submit();
}

// When PolicyBar fires `@commit`, persist (if we have a slug) or mutate
// the local draft (if we're still composing a fresh snippet). Lifetime is
// handled specially: the bar emits a preset key, we feed `lifetime_seconds`
// to the API which re-stamps `expires_at = now() + lifetime` server-side.
const savingSettings = ref(false);
async function onPolicyCommit(patch: {
  visibility?: Visibility;
  lifetimeKey?: LifetimeKey;
  burnAfterRead?: boolean;
}) {
  if (!editingSlug.value) {
    // Creating new — drive the local refs that the PolicyBar / publish
    // modal display.
    if (patch.lifetimeKey !== undefined) lifetimeKey.value = patch.lifetimeKey;
    return;
  }
  savingSettings.value = true;
  try {
    const apiPatch: {
      visibility?: Visibility;
      lifetime_seconds?: number | null;
      burn_after_read?: boolean;
    } = {};
    if (patch.visibility !== undefined) apiPatch.visibility = patch.visibility;
    if (patch.lifetimeKey !== undefined) {
      apiPatch.lifetime_seconds = LIFETIME_SECONDS[patch.lifetimeKey];
    }
    if (patch.burnAfterRead !== undefined) apiPatch.burn_after_read = patch.burnAfterRead;
    const updated = await api.updateSnippetSettings(editingSlug.value, apiPatch);
    visibility.value = updated.visibility;
    burnAfterRead.value = updated.burn_after_read;
    serverExpiresAt.value = updated.expires_at ?? null;
    toast.success('settings updated');
  } catch (e) {
    toast.error(e instanceof HttpError ? e.error.message : 'settings update failed');
  } finally {
    savingSettings.value = false;
  }
}

const showSize = computed(() => new Blob([body.value]).size);
const isOverLimit = computed(() => showSize.value > 1_048_576);

// ─────────────────────────────────────────────────────────────────────────
// Why we don't use a real editor (Monaco / CodeMirror 6)
//
// We're a snippet host — users paste arbitrary code and expect colors to
// "just appear". That requires auto-detection across ~200 languages, which
// hljs gives us for free but no production editor library does:
//   - CodeMirror 6 ships ~30 first-party language grammars and has no
//     auto-detect; the caller must pass an explicit language extension.
//   - Monaco is similar (TextMate grammars, no detection) and is ~3 MB.
//
// So the editor here stays as `<textarea>` + a transparent overlay that
// renders hljs output underneath. The pieces below — cached html prefix,
// append/backspace fast paths, span-aware truncation — exist to fight the
// "colors flash white on every keystroke" problem that the naive overlay
// approach has. They're not pretty, but they're the right trade-off for
// "any language, no user intervention" until that constraint changes.
//
// How the cache works
//
// `cachedBody` is the body the worker last highlighted, `cachedHtml` the
// html it returned. On the next change:
//   - append   → keep cachedHtml as the colored prefix, escape the new tail
//   - backspace → walk cachedHtml char-by-char and stop at the new length,
//                 re-closing any spans we were inside (no color loss)
//   - other   → escape everything for one frame, worker repaints ~150 ms
//               later. Rare while typing.
// ─────────────────────────────────────────────────────────────────────────
const { html: hlHtml, language: detectedLang, truncated: hlTruncated, highlight } = useHighlight();
const overlayHtml = ref('');
const markdownHtml = ref('');
let mdDebounceHandle: number | null = null;

// `cachedBody` is the body the worker last highlighted; `cachedHtml` is the
// html it returned. When the user types more, we reuse `cachedHtml` for
// every character of the old prefix, splice escaped text for the new tail.
// On a non-append edit (delete or middle insert) the cache is no longer a
// safe prefix and we fall back to escaping the whole body — that's the
// only case left where the colors briefly disappear, but it's per-edit, not
// per-keystroke.
let cachedBody = '';
let cachedHtml = '';

function escapeHtml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
}

// Truncate a hljs-highlighted html string so its rendered text content
// equals the first `charLimit` characters of the original source. Tags
// contribute zero visible characters; html entities like `&lt;` count as
// one. Any tokens that were open at the cut point get re-closed so the
// result is well-formed and won't bleed colors into the rest of the page.
//
// Used on backspace (and other "delete from the end" edits) so that
// existing colored chars stay colored — we don't have to fall back to
// escaping the whole body for one frame.
function truncateHighlightedHtml(html: string, charLimit: number): string {
  if (charLimit <= 0) return '';
  if (charLimit >= html.length) return html;
  let out = '';
  let visible = 0;
  const open: string[] = [];
  let i = 0;
  while (i < html.length && visible < charLimit) {
    const ch = html[i];
    if (ch === '<') {
      const end = html.indexOf('>', i);
      if (end === -1) break; // malformed, bail
      const tag = html.slice(i, end + 1);
      out += tag;
      if (tag.startsWith('</')) {
        open.pop();
      } else if (!tag.endsWith('/>')) {
        const name = tag.match(/^<\s*([a-zA-Z][\w-]*)/);
        if (name) open.push(name[1]);
      }
      i = end + 1;
    } else if (ch === '&') {
      const semi = html.indexOf(';', i);
      if (semi === -1 || semi - i > 10) {
        // Not an entity, treat as a literal `&`.
        out += ch;
        visible++;
        i++;
      } else {
        out += html.slice(i, semi + 1);
        visible++;
        i = semi + 1;
      }
    } else {
      out += ch;
      visible++;
      i++;
    }
  }
  // Re-close anything that was still open at the cut point so we don't
  // leak token spans (and their colors) into whatever the worker reply
  // splices in afterwards.
  while (open.length > 0) {
    out += `</${open.pop()}>`;
  }
  return out;
}

const textareaRef = ref<HTMLTextAreaElement | null>(null);
const overlayRef = ref<HTMLPreElement | null>(null);
const gutterInnerRef = ref<HTMLDivElement | null>(null);

// Number of logical lines, mirroring the live `body` ref. Used to render the
// line-number gutter beside the editor (matches screens.jsx EditorLoggedIn,
// where each line gets a single faint number to its left). Long wrapped
// lines still count as one — the design accepts the slight misalignment.
const lineCount = computed(() => Math.max(1, body.value.split('\n').length));

watch([body, kind], ([newBody, newKind], oldValues) => {
  const oldKind = oldValues?.[1];
  // Switching language modes invalidates the cache — the same body would
  // tokenize differently as code vs html, and we don't want the previous
  // mode's tokens to leak through as a stale prefix.
  if (oldKind !== newKind) {
    cachedBody = '';
    cachedHtml = '';
  }

  if (newKind === 'code' || newKind === 'html') {
    if (newBody.length === 0) {
      overlayHtml.value = '';
      cachedBody = '';
      cachedHtml = '';
    } else if (newBody === cachedBody) {
      // No textual change — usually a kind flip back to a previously seen
      // body. Reuse the cached html as-is.
      overlayHtml.value = cachedHtml;
    } else if (cachedBody !== '' && newBody.startsWith(cachedBody)) {
      // Append case (the common one while typing): paint the colored
      // prefix from the cache and append the new tail as escaped text.
      // The worker reply will recolor the tail when it lands.
      overlayHtml.value = cachedHtml + escapeHtml(newBody.slice(cachedBody.length));
    } else if (cachedBody.startsWith(newBody)) {
      // Backspace / delete-from-end case. The new body is a prefix of the
      // cached body, so the cached html — once trimmed to the new visible
      // length — still has correctly-colored tokens for everything that
      // remains. No "white flash" on backspace.
      overlayHtml.value = truncateHighlightedHtml(cachedHtml, newBody.length);
    } else {
      // Middle insert/delete or paste-replace. The cached token boundaries
      // no longer line up with the new body, so we briefly escape the
      // whole thing — visible for one frame until the worker re-highlights
      // from scratch.
      overlayHtml.value = escapeHtml(newBody);
    }
    highlight(newBody, newKind === 'html' ? 'html' : undefined);
    return;
  }
  if (newKind === 'markdown') {
    if (mdDebounceHandle !== null) clearTimeout(mdDebounceHandle);
    mdDebounceHandle = window.setTimeout(() => {
      markdownHtml.value = renderMarkdown(newBody);
    }, 120);
    return;
  }
});

// Worker reply lands here — swap in the fully-highlighted html and update
// the cache so the next keystroke can reuse this as its colored prefix.
// The reply is keyed to the latest sent body by useHighlight's internal
// token check, so `body.value` is the right thing to cache against.
watch(hlHtml, (h) => {
  if (kind.value !== 'code' && kind.value !== 'html') return;
  overlayHtml.value = h;
  cachedBody = body.value;
  cachedHtml = h;
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
      // Surface the existing policy so the pills aren't lying. While
      // editing, clicks on the pills hit /settings — see onPolicyCommit.
      visibility.value = s.visibility;
      burnAfterRead.value = s.burn_after_read;
      serverExpiresAt.value = s.expires_at ?? null;
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
      const seconds = LIFETIME_SECONDS[lifetimeKey.value];
      const created = await api.createSnippet({
        type: kind.value,
        name: name.value.trim() || undefined,
        body: body.value,
        visibility: visibility.value,
        lifetime_seconds: seconds ?? undefined,
        burn_after_read: burnAfterRead.value,
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
    // The keyboard shortcut comes from a hardware keyboard, i.e. desktop.
    // Submit directly — the modal-vs-inline gating in onPublishClick is for
    // the touch-button path, not for ⌘↵.
    submit();
  }
}

// Mirror the textarea's scroll into the absolutely-positioned overlay AND
// the line-number gutter so the highlighted text and the line numbers stay
// glued to the typed text. The overlay uses scrollTop directly; the gutter
// is just a tape of static numbers so we shift it with translateY instead.
function syncScroll() {
  if (!textareaRef.value) return;
  const top = textareaRef.value.scrollTop;
  if (overlayRef.value) {
    overlayRef.value.scrollTop = top;
    overlayRef.value.scrollLeft = textareaRef.value.scrollLeft;
  }
  if (gutterInnerRef.value) {
    gutterInnerRef.value.style.transform = `translateY(${-top}px)`;
  }
}

// Markdown is the only mode that still gets a side-by-side preview pane.
// Code and html now render syntax-highlighted in-place under the textarea, so
// a separate preview would be redundant noise.
const splitPreview = computed(() => kind.value === 'markdown');
const inlineHighlight = computed(() => kind.value === 'code' || kind.value === 'html');

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
    <!-- Editor takes the full viewport minus the TopBar (52px) and, on mobile,
         the bottom tab bar (~68px including safe-area inset). Three rows:
         toolbar / body / statusline. The body grows; nothing overlaps. -->
    <div class="flex flex-col h-[calc(100vh-52px-68px)] md:h-[calc(100vh-52px)]">
      <!-- toolbar — wraps on mobile so the type tabs + filename + publish
           button fit on a phone without horizontal overflow. -->
      <div class="flex flex-wrap md:flex-nowrap items-center justify-between gap-y-2 px-4 md:px-7 py-2.5 border-b border-border text-[12px] shrink-0">
        <!-- Type tabs are click-to-switch when creating, but locked while
             editing an existing snippet — PATCH /api/v1/snippets/:slug only
             updates body/name, so letting the user "change type" in the UI
             would silently drop their tab choice and re-render the old type
             after save. -->
        <div
          class="flex gap-1.5"
          :title="editingSlug ? `type can't be changed after publishing` : ''"
        >
          <button
            v-for="t in (['code', 'markdown', 'html'] as const)"
            :key="t"
            :disabled="!!editingSlug && kind !== t"
            :class="[
              'px-3 md:px-3.5 py-1.5 rounded-sm border transition-colors',
              kind === t
                ? 'bg-border text-text border-border-strong'
                : editingSlug
                  ? 'text-text-faint border-transparent cursor-not-allowed'
                  : 'text-text-muted border-transparent hover:text-text',
            ]"
            @click="kind = t"
          >{{ t === 'markdown' ? 'md' : t }}</button>
          <span v-if="editingSlug" class="hidden md:inline self-center text-[10px] text-text-faint ml-2">
            · type locked when editing
          </span>
        </div>
        <!-- Right-hand cluster. On mobile we keep just the publish button —
             filename and the decorative chips would push the publish CTA off
             screen. The filename is moved to its own row below the toolbar. -->
        <div class="flex gap-2 md:gap-3 items-center order-3 md:order-none w-full md:w-auto">
          <input
            v-model="name"
            placeholder="filename · optional"
            class="bg-bg-deep border border-border rounded-sm px-2.5 py-1 text-[12px] text-text focus:outline-none focus:border-accent flex-1 md:flex-none md:w-56"
          />
          <!-- Sharing-policy pills. Inline on desktop, hidden on mobile —
               mobile users get the same state via the publish modal
               triggered by the publish button below. While editing an
               existing snippet, the pills stay live: settings are mutable
               post-publish via the same /settings endpoint the viewer uses,
               so changes save on click. (For *new* snippets the pills just
               stage the values until publish is tapped.) -->
          <div class="hidden md:flex">
            <PolicyBar
              v-model:visibility="visibility"
              v-model:burn-after-read="burnAfterRead"
              :mode="editingSlug ? 'remote' : 'inline'"
              :pending="savingSettings"
              :expires-at="displayExpiresAt"
              @commit="onPolicyCommit"
            />
          </div>
          <span :class="['hidden md:inline text-[11px]', isOverLimit ? 'text-danger' : 'text-text-muted']">{{ showSize.toLocaleString() }} b</span>
          <span class="hidden md:inline w-px h-4 bg-border-strong" />
          <button
            :disabled="submitting || isOverLimit"
            class="bg-accent text-bg-deep font-semibold px-3 md:px-3.5 py-1 text-[12px] rounded-sm hover:opacity-90 disabled:opacity-30 shrink-0"
            @click="onPublishClick"
          >{{ submitting ? '…' : editingSlug ? 'save' : 'publish' }}<span class="hidden md:inline">{{ submitting ? '' : ' ⌘↵' }}</span></button>
        </div>
      </div>

      <!-- body — flex row of [gutter | textarea-area | (markdown preview)].
           The gutter shows line numbers in text-faint, mirroring screens.jsx
           EditorLoggedIn: one digit per logical newline, with `paddingRight: 18`.
           Markdown stacks vertically on mobile (textarea on top, preview below),
           horizontally on desktop. Code/html omit the preview entirely. -->
      <div :class="[
        'flex flex-1 min-h-0',
        splitPreview ? 'flex-col md:flex-row' : 'flex-row',
      ]">
        <!-- Line gutter (code + html only). The inner div is a static tape of
             numbers; it gets translateY'd by syncScroll so the numbers track
             the textarea's scroll position without firing reflows. -->
        <div
          v-if="inlineHighlight"
          class="shrink-0 overflow-hidden bg-bg select-none"
          aria-hidden="true"
        >
          <div
            ref="gutterInnerRef"
            class="pl-4 md:pl-7 pr-3 md:pr-[18px] py-4 md:py-5 text-text-faint text-right text-[13px] font-mono leading-relaxed will-change-transform"
          >
            <div v-for="i in lineCount" :key="i">{{ i }}</div>
          </div>
        </div>
        <!-- Textarea + highlight overlay. Both elements share the same
             padding/box so the colored characters underneath line up with
             the textarea's caret. With a gutter present we drop the left
             padding to a single space — the gutter already provides the
             visual margin. -->
        <div :class="['relative min-h-0 overflow-hidden flex-1 min-w-0', splitPreview ? 'border-b md:border-b-0 md:border-r border-border' : '']">
          <pre
            v-if="inlineHighlight"
            ref="overlayRef"
            class="absolute inset-0 m-0 pl-2 pr-4 md:pr-7 py-4 md:py-5 overflow-hidden pointer-events-none text-[13px] font-mono leading-relaxed whitespace-pre-wrap break-words text-text"
            aria-hidden="true"
          ><code class="hljs" v-html="overlayHtml || '&nbsp;'" /></pre>
          <textarea
            ref="textareaRef"
            v-model="body"
            spellcheck="false"
            wrap="soft"
            :class="[
              'absolute inset-0 w-full h-full bg-transparent py-4 md:py-5 text-[13px] font-mono leading-relaxed resize-none focus:outline-none whitespace-pre-wrap break-words placeholder:text-text-faint',
              inlineHighlight ? 'text-transparent pl-2 pr-4 md:pr-7' : 'text-text px-4 md:px-7',
            ]"
            :style="inlineHighlight ? 'caret-color: var(--color-text);' : ''"
            :placeholder="placeholder"
            @keydown="handleKeydown"
            @scroll="syncScroll"
            @input="syncScroll"
          />
        </div>

        <div v-if="kind === 'markdown'" class="overflow-auto px-4 md:px-7 py-5 md:py-7 bg-bg-deep flex-1 min-h-0 min-w-0">
          <div class="text-[10px] tracking-widest uppercase text-text-muted mb-3.5">preview</div>
          <article class="md-preview" v-html="markdownHtml || '<p style=\'color:var(--color-text-faint)\'>start writing — preview shows here.</p>'" />
        </div>
      </div>

      <!-- statusline (its own row, no overlap with the textarea) -->
      <div class="flex items-center justify-between px-4 md:px-7 py-2 border-t border-border text-[10px] md:text-[11px] shrink-0">
        <div class="flex gap-3 text-text-faint min-w-0">
          <span class="truncate"><span class="hidden md:inline">⌘+enter to publish</span><span class="md:hidden">{{ showSize.toLocaleString() }} b</span></span>
          <span v-if="kind === 'code' && detectedLang && !hlTruncated" class="text-text-muted truncate">· {{ detectedLang }}</span>
          <span v-if="hlTruncated" class="text-warn truncate">· hl off · large file</span>
        </div>
        <div v-if="error" class="text-danger truncate ml-2">{{ error }}</div>
      </div>
    </div>
    <!-- Mobile-only publish modal. Shares state with the inline desktop
         controls via v-model — flipping visibility/lifetime/burn here also
         updates the (hidden) inline selects, and vice versa. The desktop
         publish button bypasses this modal via `onPublishClick`. -->
    <PublishOptionsModal
      v-model:open="showPublishOptions"
      v-model:visibility="visibility"
      v-model:lifetime-key="lifetimeKey"
      v-model:burn-after-read="burnAfterRead"
      :submitting="submitting"
      @publish="submitFromModal"
    />
  </Shell>
</template>
