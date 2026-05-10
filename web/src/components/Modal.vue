<script setup lang="ts">
import { onBeforeUnmount, watch } from 'vue';

const props = defineProps<{
  open: boolean;
  title?: string;
  danger?: boolean;
}>();

const emit = defineEmits<{
  (e: 'update:open', value: boolean): void;
  (e: 'confirm'): void;
}>();

function close() {
  emit('update:open', false);
}

function onKey(e: KeyboardEvent) {
  if (e.key === 'Escape' && props.open) {
    e.stopPropagation();
    close();
  }
}

// Keep the listener live whenever the modal is mounted — Vue's <Teleport> keeps
// us in the tree even when not open, so we attach once and gate on `open`
// inside the handler.
watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) document.addEventListener('keydown', onKey);
    else document.removeEventListener('keydown', onKey);
  },
  { immediate: true },
);

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKey);
});
</script>

<template>
  <Teleport to="body">
    <Transition name="modal">
      <div
        v-if="open"
        class="fixed inset-0 z-50 flex items-center justify-center bg-bg/80 backdrop-blur-sm"
        @mousedown.self="close"
      >
        <div
          class="bg-bg-deep border border-border-strong rounded-sm w-[440px] max-w-[calc(100vw-2rem)] relative overflow-hidden"
          @mousedown.stop
        >
          <!-- Accent stripe on the left edge — mint normally, rose for danger. -->
          <span
            :class="[
              'absolute left-0 top-0 bottom-0 w-px',
              danger ? 'bg-danger' : 'bg-accent',
            ]"
          />

          <div class="flex items-start justify-between px-5 pt-4 pb-3 border-b border-border">
            <div class="text-[13px] text-text">
              <slot name="header">{{ title }}</slot>
            </div>
            <button
              type="button"
              class="text-text-muted hover:text-text text-[14px] leading-none -mt-0.5 -mr-1 px-1"
              aria-label="close"
              @click="close"
            >×</button>
          </div>

          <div class="px-5 py-4 text-[12px] text-text-dim leading-relaxed">
            <slot />
          </div>

          <div class="flex justify-end items-center gap-2 px-4 py-3 border-t border-border bg-bg/40">
            <slot name="actions">
              <button
                type="button"
                class="text-text-muted hover:text-text px-3 py-1.5 text-[12px]"
                @click="close"
              >cancel</button>
              <button
                v-if="danger"
                type="button"
                class="bg-danger/10 text-danger border border-danger-border rounded-sm px-3 py-1.5 text-[12px] hover:bg-danger/20"
                @click="emit('confirm')"
              >confirm</button>
              <button
                v-else
                type="button"
                class="bg-accent text-bg-deep font-semibold rounded-sm px-3 py-1.5 text-[12px] hover:opacity-90"
                @click="emit('confirm')"
              >confirm</button>
            </slot>
          </div>
        </div>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.12s ease;
}
.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}
</style>
