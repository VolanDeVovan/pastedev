// Shared lifetime presets used by PolicyBar, EditorView, and the three
// snippet viewers. Lives outside any .vue SFC because Vue's `<script setup>`
// compiler chokes on re-exporting non-type values across files in some
// configurations — keeping them in a plain .ts module sidesteps that.

export type LifetimeKey = 'never' | '15m' | '1h' | '1d' | '1w';

export const LIFETIME_SECONDS: Record<LifetimeKey, number | null> = {
  never: null,
  '15m': 15 * 60,
  '1h': 60 * 60,
  '1d': 24 * 60 * 60,
  '1w': 7 * 24 * 60 * 60,
};

export const LIFETIME_LABEL: Record<LifetimeKey, string> = {
  never: 'never',
  '15m': '15 min',
  '1h': '1 hour',
  '1d': '1 day',
  '1w': '1 week',
};

