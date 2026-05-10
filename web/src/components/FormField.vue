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
  rows?: number;
}>();
const emit = defineEmits<{ (e: 'update:modelValue', v: string): void }>();
const attrs = useAttrs();
const id = computed(() => `f-${props.label.toLowerCase().replace(/\W+/g, '-')}-${Math.random().toString(36).slice(2, 7)}`);
</script>

<template>
  <label :for="id" class="block mb-3.5">
    <div class="flex justify-between items-baseline mb-1.5">
      <span class="text-[11px] text-text-muted">{{ label }}</span>
      <span v-if="hint" class="text-[10px] text-text-faint">{{ hint }}</span>
    </div>
    <textarea
      v-if="rows"
      :id="id"
      :value="modelValue"
      :placeholder="placeholder"
      :required="required"
      :rows="rows"
      v-bind="attrs"
      class="w-full bg-bg-deep border border-border rounded-sm px-3 py-2.5 text-[13px] text-text font-mono focus:outline-none focus:border-accent transition-colors resize-none leading-snug"
      @input="emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
    />
    <input
      v-else
      :id="id"
      :type="type ?? 'text'"
      :value="modelValue"
      :placeholder="placeholder"
      :autocomplete="autocomplete"
      :required="required"
      v-bind="attrs"
      class="w-full bg-bg-deep border border-border rounded-sm px-3 py-2.5 text-[13px] text-text font-mono focus:outline-none focus:border-accent transition-colors"
      @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
    />
  </label>
</template>
