/// Tauri serializes `AppError` as `{ code, message, ... }`. Older paths (or
/// non-Tauri throws) still surface as plain strings. This helper normalises
/// both into a user-displayable string without leaking `[object Object]`.
export function errorMessage(e: unknown): string {
  if (e == null) return '';
  if (typeof e === 'string') return e;
  if (typeof e === 'object' && 'message' in (e as Record<string, unknown>)) {
    const msg = (e as { message?: unknown }).message;
    if (typeof msg === 'string') return msg;
  }
  return String(e);
}

/// Structured-error matching. Backend error `code` is stable and decoupled
/// from the human-readable message, so feature code should match on `code`
/// instead of regexing the message.
export function errorCode(e: unknown): string | undefined {
  if (e && typeof e === 'object' && 'code' in (e as Record<string, unknown>)) {
    const code = (e as { code?: unknown }).code;
    if (typeof code === 'string') return code;
  }
  return undefined;
}

/// Pull a structured field (e.g. `count` from EXTERNAL_IN_UNDO_RANGE) off a
/// Tauri error. Returns undefined if absent so callers can fall back.
export function errorField<T = unknown>(e: unknown, key: string): T | undefined {
  if (e && typeof e === 'object' && key in (e as Record<string, unknown>)) {
    return (e as Record<string, T>)[key];
  }
  return undefined;
}
