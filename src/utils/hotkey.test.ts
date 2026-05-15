import { describe, it, expect } from 'vitest';
import { translateKeyEvent, type KeyEventLike } from './hotkey';

/** 构造一个 KeyEventLike，修饰键默认全 false。 */
function ev(partial: Partial<KeyEventLike>): KeyEventLike {
  return {
    key: '',
    code: '',
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ...partial,
  };
}

describe('translateKeyEvent', () => {
  it('translates a single letter with one modifier', () => {
    expect(translateKeyEvent(ev({ key: 't', code: 'KeyT', ctrlKey: true }))).toBe('Ctrl+T');
  });

  it('translates multiple modifiers in a fixed Ctrl+Shift+Alt+Super order', () => {
    const e = ev({ key: 't', code: 'KeyT', altKey: true, ctrlKey: true, shiftKey: true });
    expect(translateKeyEvent(e)).toBe('Ctrl+Shift+Alt+T');
  });

  it('returns null for a modifier-only event (no main key yet)', () => {
    expect(translateKeyEvent(ev({ key: 'Control', code: 'ControlLeft', ctrlKey: true }))).toBeNull();
    expect(translateKeyEvent(ev({ key: 'Shift', code: 'ShiftLeft', shiftKey: true }))).toBeNull();
  });

  it('translates digit and function keys', () => {
    expect(translateKeyEvent(ev({ key: '1', code: 'Digit1', ctrlKey: true }))).toBe('Ctrl+1');
    expect(translateKeyEvent(ev({ key: 'F5', code: 'F5', altKey: true }))).toBe('Alt+F5');
  });

  it('returns null for Escape (used to cancel recording, never a binding)', () => {
    expect(translateKeyEvent(ev({ key: 'Escape', code: 'Escape', ctrlKey: true }))).toBeNull();
  });

  it('returns null for an unsupported main key', () => {
    expect(translateKeyEvent(ev({ key: 'Dead', code: 'IntlBackslash', ctrlKey: true }))).toBeNull();
  });

  it('maps meta key to Super', () => {
    expect(translateKeyEvent(ev({ key: 'k', code: 'KeyK', metaKey: true }))).toBe('Super+K');
  });
});
