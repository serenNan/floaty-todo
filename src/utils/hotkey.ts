/**
 * translateKeyEvent 只读取 KeyboardEvent 的这几个字段。抽成接口让单测能
 * 直接构造普通对象，不必造真实 DOM 事件。
 */
export interface KeyEventLike {
  key: string;
  code: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
}

/** 修饰键自身的 `key` 值 —— 单独按这些键不构成一个合法绑定。 */
const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta']);

/** 少数有名字的非字母数字主键，映射到 Tauri accelerator 接受的 token。 */
const NAMED_KEYS: Record<string, string> = {
  Space: 'Space',
  Enter: 'Enter',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
};

/** 把 `event.code` 翻成 Tauri accelerator 主键 token，不支持的返回 null。 */
function mainKeyToken(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3); // KeyT -> T
  if (/^Digit[0-9]$/.test(code)) return code.slice(5); // Digit1 -> 1
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code; // F5 -> F5
  return NAMED_KEYS[code] ?? null;
}

/**
 * 把一次 keydown 翻译成 Tauri accelerator 字符串（如 "Ctrl+Shift+T"）。
 * 返回 null 表示这次事件没有合法主键 —— 用户只按着修饰键，或主键不受支持，
 * 录制态应继续等下一次按键。
 */
export function translateKeyEvent(e: KeyEventLike): string | null {
  if (MODIFIER_KEYS.has(e.key)) return null;
  const main = mainKeyToken(e.code);
  if (!main) return null;
  const parts: string[] = [];
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.shiftKey) parts.push('Shift');
  if (e.altKey) parts.push('Alt');
  if (e.metaKey) parts.push('Super');
  parts.push(main);
  return parts.join('+');
}
