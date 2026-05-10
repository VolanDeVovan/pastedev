import { defineStore } from 'pinia';
import { ref } from 'vue';

export interface Toast {
  id: number;
  kind: 'info' | 'error' | 'success';
  message: string;
}

let nextId = 1;

export const useToastStore = defineStore('toast', () => {
  const items = ref<Toast[]>([]);

  function show(kind: Toast['kind'], message: string, ttl = 4000) {
    const id = nextId++;
    items.value.push({ id, kind, message });
    setTimeout(() => dismiss(id), ttl);
  }

  function dismiss(id: number) {
    items.value = items.value.filter((t) => t.id !== id);
  }

  return {
    items,
    info: (msg: string) => show('info', msg),
    error: (msg: string) => show('error', msg, 6000),
    success: (msg: string) => show('success', msg),
    dismiss,
  };
});
