<script setup lang="ts">
/// Cartoon-leaning SVG icon library. All icons share the same stroke width,
/// rounded caps/joins and `currentColor`, so they look like a coherent set
/// even when used at very different sizes. Lucide-inspired path data with
/// minor tweaks for a friendlier feel (chunkier pin head, bigger gear teeth).
///
/// Add new icons here rather than scattering raw SVG strings across components.

export type IconName =
  | 'pin'
  | 'pin-off'
  | 'settings'
  | 'refresh'
  | 'plus'
  | 'chevron-down'
  | 'chevron-right'
  | 'more-horizontal'
  | 'pencil'
  | 'rotate-ccw'
  | 'folder'
  | 'file'
  | 'trash'
  | 'sun'
  | 'moon'
  | 'monitor'
  | 'arrow-left'
  | 'loader'
  | 'check'
  | 'x';

withDefaults(defineProps<{
  name: IconName;
  size?: number | string;
}>(), { size: 16 });
</script>

<template>
  <svg
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width="1.9"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    class="floaty-icon"
  >
    <!-- Pin (active): plump, slightly tilted thumbtack with a filled head -->
    <g v-if="name === 'pin'" transform="rotate(-18 12 12)">
      <ellipse cx="12" cy="7.2" rx="5" ry="3.6" fill="currentColor" stroke="none" />
      <ellipse cx="10.5" cy="6" rx="1.4" ry="0.9" fill="white" fill-opacity="0.45" stroke="none" />
      <rect x="11" y="10.6" width="2" height="2.2" rx="0.4" fill="currentColor" stroke="none" />
      <line x1="12" y1="13" x2="12" y2="20.5" stroke="currentColor" stroke-width="2.4" />
    </g>

    <!-- Pin (off): same shape, no fill, more tilt — feels "floating" -->
    <g v-else-if="name === 'pin-off'" transform="rotate(-35 12 12)">
      <ellipse cx="12" cy="7.2" rx="4.8" ry="3.4" />
      <line x1="12" y1="10.6" x2="12" y2="20" />
    </g>

    <!-- Settings: chunky gear with a clear centre -->
    <g v-else-if="name === 'settings'">
      <circle cx="12" cy="12" r="2.8" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3h.1a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5h.1a1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8v.1a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
    </g>

    <!-- Refresh: rotate-cw arrows -->
    <g v-else-if="name === 'refresh'">
      <polyline points="21 4 21 10 15 10" />
      <path d="M3.51 15a9 9 0 0 0 14.85 3.36L21 16" />
      <polyline points="3 20 3 14 9 14" />
      <path d="M20.49 9A9 9 0 0 0 5.64 5.64L3 8" />
    </g>

    <!-- Plus -->
    <g v-else-if="name === 'plus'">
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </g>

    <!-- Chevron-down (used as caret-expanded) -->
    <polyline v-else-if="name === 'chevron-down'" points="6 9 12 15 18 9" />

    <!-- Chevron-right (caret-collapsed) -->
    <polyline v-else-if="name === 'chevron-right'" points="9 6 15 12 9 18" />

    <!-- More horizontal (⋯) -->
    <g v-else-if="name === 'more-horizontal'">
      <circle cx="12" cy="12" r="1.6" fill="currentColor" stroke="none" />
      <circle cx="5" cy="12" r="1.6" fill="currentColor" stroke="none" />
      <circle cx="19" cy="12" r="1.6" fill="currentColor" stroke="none" />
    </g>

    <!-- Pencil (edit) -->
    <g v-else-if="name === 'pencil'">
      <path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
      <line x1="14" y1="6" x2="18" y2="10" />
    </g>

    <!-- Rotate-ccw (reset) -->
    <g v-else-if="name === 'rotate-ccw'">
      <polyline points="1 4 1 10 7 10" />
      <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
    </g>

    <!-- Folder -->
    <path
      v-else-if="name === 'folder'"
      d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"
    />

    <!-- File-text -->
    <g v-else-if="name === 'file'">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="8" y1="13" x2="16" y2="13" />
      <line x1="8" y1="17" x2="14" y2="17" />
    </g>

    <!-- Trash -->
    <g v-else-if="name === 'trash'">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </g>

    <!-- Sun -->
    <g v-else-if="name === 'sun'">
      <circle cx="12" cy="12" r="4" />
      <line x1="12" y1="2" x2="12" y2="4" />
      <line x1="12" y1="20" x2="12" y2="22" />
      <line x1="4.93" y1="4.93" x2="6.34" y2="6.34" />
      <line x1="17.66" y1="17.66" x2="19.07" y2="19.07" />
      <line x1="2" y1="12" x2="4" y2="12" />
      <line x1="20" y1="12" x2="22" y2="12" />
      <line x1="6.34" y1="17.66" x2="4.93" y2="19.07" />
      <line x1="19.07" y1="4.93" x2="17.66" y2="6.34" />
    </g>

    <!-- Moon -->
    <path
      v-else-if="name === 'moon'"
      d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z"
    />

    <!-- Monitor -->
    <g v-else-if="name === 'monitor'">
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2" />
      <line x1="8" y1="21" x2="16" y2="21" />
      <line x1="12" y1="17" x2="12" y2="21" />
    </g>

    <!-- Arrow-left -->
    <g v-else-if="name === 'arrow-left'">
      <line x1="19" y1="12" x2="5" y2="12" />
      <polyline points="12 19 5 12 12 5" />
    </g>

    <!-- Loader (spin via CSS on container) -->
    <path
      v-else-if="name === 'loader'"
      d="M21 12a9 9 0 1 1-6.219-8.56"
    />

    <!-- Check -->
    <polyline v-else-if="name === 'check'" points="20 6 9 17 4 12" />

    <!-- X (close) -->
    <g v-else-if="name === 'x'">
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </g>
  </svg>
</template>

<style scoped>
.floaty-icon {
  display: block;
  flex-shrink: 0;
}
</style>
