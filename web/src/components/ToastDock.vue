<script setup lang="ts">
import { useToastStore } from '../stores/toast';

const toast = useToastStore();
</script>

<template>
  <div class="fixed bottom-4 right-4 flex flex-col gap-2 z-50">
    <transition-group name="toast">
      <div
        v-for="t in toast.items"
        :key="t.id"
        :class="[
          'px-3 py-2 text-[12px] border border-l-[3px] bg-bg-deep shadow-lg max-w-md rounded-sm',
          t.kind === 'error' ? 'border-danger-border border-l-danger text-text-dim' :
          t.kind === 'success' ? 'border-accent/40 border-l-accent text-text' :
          'border-border-strong border-l-border-strong text-text',
        ]"
      >
        {{ t.message }}
      </div>
    </transition-group>
  </div>
</template>

<style>
.toast-enter-from { opacity: 0; transform: translateY(8px); }
.toast-enter-active { transition: all 160ms ease-out; }
.toast-leave-to { opacity: 0; transform: translateY(8px); }
.toast-leave-active { transition: all 160ms ease-in; }
</style>
