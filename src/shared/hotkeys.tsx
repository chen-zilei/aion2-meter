// The global shortcuts and how they are written ("Ctrl+Shift+L"), shared by Settings and the hint text elsewhere.
import type { Hotkeys } from "./types";

export const HOTKEY_ACTIONS: { id: keyof Hotkeys; label: string; hint: string }[] = [
  { id: "lockOverlay", label: "Lock overlay", hint: "Click-through on or off" },
  { id: "toggleOverlay", label: "Show / hide overlay", hint: "" },
  { id: "newEncounter", label: "New encounter", hint: "End the current fight and start counting a new one" },
];

/** Same as `Hotkeys::default()` in src-tauri/src/settings.rs. */
export const DEFAULT_HOTKEYS: Hotkeys = { lockOverlay: "Ctrl+Shift+L", toggleOverlay: "Ctrl+Shift+O", newEncounter: "Ctrl+Shift+R" };

const MODIFIERS = { Ctrl: "Ctrl", Alt: "Alt", Shift: "Shift", Super: "Win" } as const;

// Keys the backend understands by their KeyboardEvent.code (letters and digits are handled separately).
const NAMED_KEYS = new Set([
  "Backquote", "Backslash", "BracketLeft", "BracketRight", "Comma", "Equal", "Minus", "Period", "Quote", "Semicolon",
  "Slash", "Backspace", "Enter", "Space", "Tab", "Delete", "End", "Home", "Insert", "PageDown", "PageUp", "PrintScreen",
  "ScrollLock", "Pause", "ArrowDown", "ArrowLeft", "ArrowRight", "ArrowUp", "NumpadAdd", "NumpadDecimal", "NumpadDivide",
  "NumpadEnter", "NumpadMultiply", "NumpadSubtract",
]);

const KEY_LABELS: Record<string, string> = {
  Backquote: "`", Backslash: "\\", BracketLeft: "[", BracketRight: "]", Comma: ",", Equal: "=", Minus: "-", Period: ".",
  Quote: "'", Semicolon: ";", Slash: "/", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→", ArrowUp: "↑",
  PageDown: "Page Down", PageUp: "Page Up", PrintScreen: "Print Screen", ScrollLock: "Scroll Lock",
  NumpadAdd: "Num +", NumpadDecimal: "Num .", NumpadDivide: "Num /", NumpadEnter: "Num Enter",
  NumpadMultiply: "Num *", NumpadSubtract: "Num -",
};

/** Keys that are fine on their own; anything else needs a modifier so it doesn't steal a key the game uses. */
const SOLO_OK = /^(F\d{1,2}|Pause|ScrollLock)$/;

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit\d$/.test(code)) return code.slice(5);
  if (/^Numpad\d$/.test(code) || /^F([1-9]|1\d|2[0-4])$/.test(code) || NAMED_KEYS.has(code)) return code;
  return null;
}

export function modifiersOf(e: KeyboardEvent): string[] {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push("Ctrl");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (e.metaKey) mods.push("Super");
  return mods;
}

export const isModifierKey = (e: KeyboardEvent) => ["Control", "Alt", "Shift", "Meta", "OS"].includes(e.key);

/** The shortcut a key press makes, or why it can't be one. */
export function shortcutFrom(e: KeyboardEvent): { keys: string } | { error: string } {
  const key = keyName(e.code);
  if (!key) return { error: "That key can't be used for a shortcut." };
  const mods = modifiersOf(e);
  if (!mods.length && !SOLO_OK.test(key)) return { error: "Hold Ctrl, Alt or Shift with that key, so the game still gets it." };
  return { keys: [...mods, key].join("+") };
}

/** "Ctrl+Shift+ArrowUp" as the user reads it: ["Ctrl", "Shift", "↑"]. */
export function keyLabels(keys: string): string[] {
  return keys
    .split("+")
    .filter(Boolean)
    .map((k) => MODIFIERS[k as keyof typeof MODIFIERS] ?? KEY_LABELS[k] ?? k.replace(/^Numpad(\d)$/, "Num $1"));
}

/** Two spellings of the same shortcut compare equal, whatever the modifier order or case. */
export function sameShortcut(a: string, b: string): boolean {
  const norm = (k: string) => {
    const parts = k.toUpperCase().split("+").filter(Boolean);
    const key = parts.pop() ?? "";
    return [...parts.sort(), key.replace(/^KEY(?=[A-Z]$)|^DIGIT(?=\d$)/, "")].join("+");
  };
  return !!a && !!b && norm(a) === norm(b);
}

export function Keys({ keys }: { keys: string }) {
  if (!keys) return <span className="dim">Not set</span>;
  return (
    <span className="keys">
      {keyLabels(keys).map((k, i) => (
        <span key={i}>
          {i > 0 && "+"}
          <kbd>{k}</kbd>
        </span>
      ))}
    </span>
  );
}

/** A shortcut as plain text, for tooltips: "Ctrl+Shift+L". */
export const keysText = (keys: string) => keyLabels(keys).join("+") || "no shortcut";
