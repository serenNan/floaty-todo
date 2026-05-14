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
      "Reveal in file manager" — open folder with a magnifier inside.
      Reads as "look inside this folder" at 14px without being literal.
    -->
    <path
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linejoin="round"
      d="M3 7a2 2 0 0 1 2-2h4l2 2.5h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"
    />
    <circle
      cx="13"
      cy="14"
      r="2.4"
      fill="none"
      stroke="currentColor"
      stroke-width="1.8"
    />
    <line
      x1="14.9"
      y1="15.9"
      x2="17.2"
      y2="18.2"
      stroke="currentColor"
      stroke-width="1.8"
      stroke-linecap="round"
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
