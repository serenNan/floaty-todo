/// Preset accent colors offered for sources. Tailwind 500-ish hues — vivid
/// enough to be readable on both light and dark backgrounds without being
/// neon. Picked short so the swatch row fits on one line in the narrow
/// settings pane / in-header editor.
export const SOURCE_COLORS: readonly string[] = [
  '#ef4444', '#f97316', '#f59e0b', '#84cc16',
  '#10b981', '#06b6d4', '#3b82f6', '#8b5cf6', '#ec4899',
];

const HEX_RE = /^#[0-9a-fA-F]{3,8}$/;

/// Validate a color from untrusted sources (config file, prop) so it can't
/// inject arbitrary CSS through inline style. Returns the original string
/// when it's a valid hex, null otherwise.
export function safeHexColor(c: string | null | undefined): string | null {
  if (!c) return null;
  return HEX_RE.test(c) ? c : null;
}
