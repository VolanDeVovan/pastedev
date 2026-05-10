<script setup lang="ts">
import { computed, useAttrs } from 'vue';

const props = defineProps<{
  modelValue: string;
  label: string;
  type?: string;
  placeholder?: string;
  autocomplete?: string;
  required?: boolean;
  hint?: string;
}>();
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>();
const attrs = useAttrs();
const id = computed(() => `f-${props.label.toLowerCase().replace(/\W+/g, '-')}-${Math.random().toString(36).slice(2, 7)}`);
</script>

<template>
  <label :for="id" class="block">
    <span class="text-[11px] uppercase tracking-widest text-text-muted">{{ label }}</span>
    <input
      :id="id"
      :type="type ?? 'text'"
      :value="modelValue"
      :placeholder="placeholder"
      :autocomplete="autocomplete"
      :required="required"
      v-bind="attrs"
      class="mt-1.5 w-full bg-panel border border-border-strong px-3 py-2 text-sm focus:outline-none focus:border-accent transition-colors"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
    <span v-if="hint" class="block text-xs text-text-muted mt-1">{{ hint }}</span>
  </label>
</template>
