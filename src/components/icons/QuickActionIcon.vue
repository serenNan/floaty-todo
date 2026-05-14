<script setup lang="ts">
import type { QuickActionKind } from '../../types/task';

defineProps<{ kind: QuickActionKind }>();
</script>

<template>
  <!--
    Inline SVGs from simple-icons (CC0). Paths kept small so the bundle
    stays tiny; brand colours applied per-icon so the user can recognise
    them at a glance.
  -->
  <svg
    v-if="kind === 'vscode'"
    class="qa-icon vscode"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <path
      fill="currentColor"
      d="M23.15 2.587 18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"
    />
  </svg>

  <svg
    v-else-if="kind === 'terminal'"
    class="qa-icon terminal"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <!--
      Window chrome + prompt — reads as "PowerShell / Terminal" regardless
      of which emulator actually launches. Kept brand-neutral on purpose so
      future user-custom shells (cmd / bash / pwsh) still feel correct.
    -->
    <rect
      x="2"
      y="3.5"
      width="20"
      height="17"
      rx="2"
      ry="2"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
    />
    <path
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      d="M6 9.5l3 2.5-3 2.5"
    />
    <line
      x1="11.5"
      y1="15"
      x2="17"
      y2="15"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
    />
  </svg>

  <svg
    v-else-if="kind === 'claude_code'"
    class="qa-icon claude"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <!--
      Anthropic "sparkle" mark — eight-pointed star, simplified from the
      official asterisk shape so it reads cleanly at 14px.
    -->
    <path
      fill="currentColor"
      d="M12 2.5c.3 0 .55.2.62.49l1.27 5.04a3 3 0 0 0 2.08 2.08l5.04 1.27c.29.07.49.32.49.62s-.2.55-.49.62l-5.04 1.27a3 3 0 0 0-2.08 2.08l-1.27 5.04c-.07.29-.32.49-.62.49s-.55-.2-.62-.49l-1.27-5.04a3 3 0 0 0-2.08-2.08L2.99 12.62A.64.64 0 0 1 2.5 12c0-.3.2-.55.49-.62l5.04-1.27a3 3 0 0 0 2.08-2.08l1.27-5.04A.64.64 0 0 1 12 2.5z"
    />
  </svg>

  <svg
    v-else-if="kind === 'reveal'"
    class="qa-icon reveal"
    viewBox="0 0 24 24"
    aria-hidden="true"
  >
    <!--
      Chunky filled folder with a soft top-edge highlight — reads as a
      cartoon manila folder at 14px. Dropped the magnifier overlay: the
      icon-only style is more honest about the action being
      "open in file manager", and visually less cluttered.
    -->
    <path
      fill="currentColor"
      d="M2.5 7.6c0-1.1 0.9-2 2-2h4.6c0.5 0 1 0.2 1.4 0.6l1.3 1.3c0.4 0.4 0.9 0.6 1.4 0.6h6.3c1.1 0 2 0.9 2 2v8.3c0 1.1-0.9 2-2 2H4.5c-1.1 0-2-0.9-2-2V7.6z"
    />
    <!-- Top-edge highlight: a thin pale line just below the rim to imply
         the front panel folding forward. -->
    <path
      stroke="white"
      stroke-opacity="0.35"
      stroke-width="1.2"
      stroke-linecap="round"
      fill="none"
      d="M3.6 9.8h16.8"
    />
  </svg>
</template>

<style scoped>
.qa-icon {
  width: 14px;
  height: 14px;
  display: block;
  flex-shrink: 0;
}

.qa-icon.vscode  { color: #0098ff; }
.qa-icon.terminal { color: #4daa7f; }   /* PowerShell-ish blue-green */
.qa-icon.claude  { color: #d97757; }   /* Anthropic warm orange */
.qa-icon.reveal  { color: #f59e0b; }   /* Folder yellow — system-file-manager vibe */

/* Dark mode: nudge brand tones for readability on the glass surface. */
:global(:root[data-theme="dark"]) .qa-icon.vscode  { color: #29b6ff; }
:global(:root[data-theme="dark"]) .qa-icon.terminal { color: #5ec79a; }
:global(:root[data-theme="dark"]) .qa-icon.claude  { color: #e89178; }
:global(:root[data-theme="dark"]) .qa-icon.reveal  { color: #fbbf24; }
</style>
