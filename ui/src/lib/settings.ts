import { appState } from "./store.svelte";

const STORAGE_KEY = "az-plotter-settings-v1";

/// Suggested UI font stacks. The settings UI lets the user type any font
/// name too — these just provide autocomplete. Each `value` is a CSS
/// font-family stack with sensible fallbacks so a typo still renders.
export const UI_FONT_OPTIONS: Array<{ label: string; value: string }> = [
  { label: "System UI",       value: "system-ui, sans-serif" },
  { label: "Segoe UI",        value: "'Segoe UI', system-ui, sans-serif" },
  { label: "Segoe UI Variable", value: "'Segoe UI Variable', 'Segoe UI', sans-serif" },
  { label: "Calibri",         value: "Calibri, sans-serif" },
  { label: "Arial",           value: "Arial, sans-serif" },
  { label: "Helvetica",       value: "Helvetica, Arial, sans-serif" },
  { label: "Helvetica Neue",  value: "'Helvetica Neue', Arial, sans-serif" },
  { label: "Tahoma",          value: "Tahoma, sans-serif" },
  { label: "Verdana",         value: "Verdana, sans-serif" },
  { label: "Trebuchet MS",    value: "'Trebuchet MS', sans-serif" },
  { label: "Inter",           value: "Inter, system-ui, sans-serif" },
  { label: "Roboto",          value: "Roboto, system-ui, sans-serif" },
  { label: "Open Sans",       value: "'Open Sans', system-ui, sans-serif" },
  { label: "Noto Sans",       value: "'Noto Sans', system-ui, sans-serif" },
  { label: "Source Sans Pro", value: "'Source Sans Pro', system-ui, sans-serif" },
  { label: "Georgia",         value: "Georgia, 'Times New Roman', serif" },
  { label: "Times New Roman", value: "'Times New Roman', Times, serif" },
  { label: "Cambria",         value: "Cambria, Georgia, serif" },
  { label: "Garamond",        value: "Garamond, serif" },
  { label: "Book Antiqua",    value: "'Book Antiqua', Palatino, serif" },
];

export const MONO_FONT_OPTIONS: Array<{ label: string; value: string }> = [
  { label: "UI Monospace",    value: "ui-monospace, Menlo, Consolas, monospace" },
  { label: "Cascadia Code",   value: "'Cascadia Code', Consolas, monospace" },
  { label: "Cascadia Mono",   value: "'Cascadia Mono', Consolas, monospace" },
  { label: "Consolas",        value: "Consolas, monospace" },
  { label: "Courier New",     value: "'Courier New', Courier, monospace" },
  { label: "Lucida Console",  value: "'Lucida Console', Monaco, monospace" },
  { label: "Lucida Sans Typewriter", value: "'Lucida Sans Typewriter', monospace" },
  { label: "JetBrains Mono",  value: "'JetBrains Mono', Consolas, monospace" },
  { label: "Fira Code",       value: "'Fira Code', Consolas, monospace" },
  { label: "Source Code Pro", value: "'Source Code Pro', Consolas, monospace" },
  { label: "IBM Plex Mono",   value: "'IBM Plex Mono', Consolas, monospace" },
  { label: "Inconsolata",     value: "Inconsolata, Consolas, monospace" },
  { label: "Menlo",           value: "Menlo, Consolas, monospace" },
  { label: "Monaco",          value: "Monaco, Consolas, monospace" },
  { label: "DejaVu Sans Mono",value: "'DejaVu Sans Mono', Consolas, monospace" },
  { label: "Hack",            value: "Hack, Consolas, monospace" },
  { label: "Roboto Mono",     value: "'Roboto Mono', Consolas, monospace" },
  { label: "Victor Mono",     value: "'Victor Mono', Consolas, monospace" },
];

export function loadSettings(): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw);
    if (typeof parsed?.uiFont === "string") appState.settings.uiFont = parsed.uiFont;
    if (typeof parsed?.monoFont === "string") appState.settings.monoFont = parsed.monoFont;
    if (typeof parsed?.fontSize === "number") {
      appState.settings.fontSize = clampSize(parsed.fontSize);
    }
  } catch { /* ignore — user will just see defaults */ }
}

export function saveSettings(): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(appState.settings));
  } catch { /* quota/privacy mode */ }
}

/// Push the current settings into CSS custom properties on <html>. Components
/// read these via `var(--app-ui-font)` etc.
export function applySettingsToDocument(): void {
  const s = appState.settings;
  const ui = normalizeFontStack(s.uiFont);
  const mono = normalizeFontStack(s.monoFont);
  const root = document.documentElement.style;
  root.setProperty("--app-ui-font",   ui);
  root.setProperty("--app-mono-font", mono);
  root.setProperty("--app-font-size", `${s.fontSize}px`);
}

/// CSS needs multi-word font names quoted, otherwise `Cascadia Code` is
/// parsed as two separate families and both fail. Split on commas, trim,
/// and quote any unquoted name that has a space.
export function normalizeFontStack(stack: string): string {
  return stack
    .split(",")
    .map(s => s.trim())
    .filter(s => s.length > 0)
    .map(s => {
      if (/^["'].*["']$/.test(s)) return s;
      if (s.includes(" ")) return `'${s.replace(/'/g, "\\'")}'`;
      return s;
    })
    .join(", ");
}

export function clampSize(n: number): number {
  if (!Number.isFinite(n)) return 12;
  return Math.max(9, Math.min(22, Math.round(n)));
}
