import { computed, onBeforeUnmount, ref, watch, type Ref } from 'vue';
import type { Snippet } from '../api';
import { BURN_AFTER_READ_WINDOW_SECONDS } from '../api/types';

export interface CountdownState {
  /// Effective expiry timestamp the countdown ticks toward, or null when the
  /// snippet has no fixed expiry / hasn't been viewed yet.
  expiresAt: Ref<Date | null>;
  /// Whole seconds remaining, clamped to 0. null when no countdown is active.
  secondsLeft: Ref<number | null>;
  /// Human label like `4m 12s` or `1d 3h`.
  label: Ref<string>;
  /// True once the countdown has reached zero. Stays true after.
  expired: Ref<boolean>;
  /// "burn after read" + not yet viewed — shows a hint rather than a ticking clock.
  burnArmed: Ref<boolean>;
}

/// Drives the countdown shown in the snippet viewer toolbar. Subscribes to a
/// reactive `snippet` ref so the timer reattaches after the slug-load promise
/// resolves. Ticks every second; cleans up on unmount.
export function useSnippetCountdown(snippet: Ref<Snippet | null>): CountdownState {
  const now = ref(Date.now());
  let handle: number | null = null;

  const expiresAt = computed<Date | null>(() => {
    const s = snippet.value;
    if (!s?.expires_at) return null;
    return new Date(s.expires_at);
  });

  const burnArmed = computed(() => {
    const s = snippet.value;
    return !!s && s.burn_after_read && !s.first_viewed_at;
  });

  const secondsLeft = computed<number | null>(() => {
    const exp = expiresAt.value;
    if (!exp) return null;
    return Math.max(0, Math.floor((exp.getTime() - now.value) / 1000));
  });

  const expired = computed(() => secondsLeft.value !== null && secondsLeft.value <= 0);

  const label = computed(() => {
    if (burnArmed.value) {
      // Pre-view hint: shows the upper bound (~15min) so the reader can
      // calibrate, even though the timer hasn't started ticking yet.
      return `burns ${formatDuration(BURN_AFTER_READ_WINDOW_SECONDS)} after first view`;
    }
    const left = secondsLeft.value;
    if (left == null) return '';
    if (left <= 0) return 'expired';
    return formatDuration(left);
  });

  // Manage the 1-second tick lifecycle. The interval only runs while we
  // actually have a countdown to tick — saves a wakeup-per-second on
  // long-lived (never-expires) pages.
  function ensureTicking() {
    const needs = expiresAt.value !== null && !expired.value;
    if (needs && handle === null) {
      handle = window.setInterval(() => {
        now.value = Date.now();
      }, 1000);
    } else if (!needs && handle !== null) {
      clearInterval(handle);
      handle = null;
    }
  }

  watch([expiresAt, expired], ensureTicking, { immediate: true });
  onBeforeUnmount(() => {
    if (handle !== null) clearInterval(handle);
  });

  return { expiresAt, secondsLeft, label, expired, burnArmed };
}

/// Compact duration label (`1w 2d`, `3h 14m`, `4m 12s`, `42s`). Drops trailing
/// zero units, never shows more than two adjacent units — that's plenty of
/// resolution for the "is this still readable" question we're answering.
export function formatDuration(totalSeconds: number): string {
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const units: Array<[string, number]> = [
    ['w', 7 * 24 * 60 * 60],
    ['d', 24 * 60 * 60],
    ['h', 60 * 60],
    ['m', 60],
    ['s', 1],
  ];
  const parts: string[] = [];
  let remaining = totalSeconds;
  for (const [label, sec] of units) {
    const n = Math.floor(remaining / sec);
    if (n > 0) {
      parts.push(`${n}${label}`);
      remaining -= n * sec;
      if (parts.length === 2) break;
    } else if (parts.length > 0) {
      // Skip zero middle units so `1d 0h 3m` becomes `1d 3m`. Stops at the
      // first two non-zero units regardless.
    }
  }
  return parts.join(' ');
}
