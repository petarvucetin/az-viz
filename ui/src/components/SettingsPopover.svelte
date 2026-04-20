<script lang="ts">
  import { appState } from "../lib/store.svelte";
  import { UI_FONT_OPTIONS, MONO_FONT_OPTIONS, clampSize, applySettingsToDocument, saveSettings } from "../lib/settings";

  let open = $state(false);
  let rootEl: HTMLDivElement | null = $state(null);

  function toggle() { open = !open; }
  function close() { open = false; }

  // Close when clicking outside. Bound only while open.
  $effect(() => {
    if (!open) return;
    const onDown = (ev: MouseEvent) => {
      if (rootEl && !rootEl.contains(ev.target as Node)) close();
    };
    // Defer one tick so the opening click doesn't immediately close us.
    const t = setTimeout(() => document.addEventListener("mousedown", onDown), 0);
    return () => { clearTimeout(t); document.removeEventListener("mousedown", onDown); };
  });

  function onFontSizeInput(e: Event) {
    const v = Number((e.target as HTMLInputElement).value);
    appState.settings.fontSize = clampSize(v);
    applyNow();
  }

  function onUiFontInput(e: Event) {
    appState.settings.uiFont = (e.target as HTMLInputElement).value;
    applyNow();
  }

  function onMonoFontInput(e: Event) {
    appState.settings.monoFont = (e.target as HTMLInputElement).value;
    applyNow();
  }

  // Direct-apply on every input — belt-and-suspenders in case reactivity
  // doesn't pick up the nested-state bind.
  function applyNow() {
    applySettingsToDocument();
    saveSettings();
  }

  function resetDefaults() {
    appState.settings.uiFont = UI_FONT_OPTIONS[0].value;
    appState.settings.monoFont = MONO_FONT_OPTIONS[0].value;
    appState.settings.fontSize = 12;
    applyNow();
  }
</script>

<div class="settings" bind:this={rootEl}>
  <button class="gear" onclick={toggle} aria-label="Settings" title="Settings">⚙</button>
  {#if open}
    <div class="popover" role="dialog" aria-label="Settings">
      <div class="grp">
        <label for="ui-font">UI font</label>
        <input
          id="ui-font"
          type="text"
          list="ui-font-list"
          spellcheck="false"
          autocomplete="off"
          value={appState.settings.uiFont}
          oninput={onUiFontInput}
          onchange={onUiFontInput}
        />
        <datalist id="ui-font-list">
          {#each UI_FONT_OPTIONS as o}
            <option value={o.value} label={o.label}></option>
          {/each}
        </datalist>
        <div class="hint">Any installed font works — type its name.</div>
      </div>
      <div class="grp">
        <label for="mono-font">Monospace font</label>
        <input
          id="mono-font"
          type="text"
          list="mono-font-list"
          spellcheck="false"
          autocomplete="off"
          value={appState.settings.monoFont}
          oninput={onMonoFontInput}
          onchange={onMonoFontInput}
        />
        <datalist id="mono-font-list">
          {#each MONO_FONT_OPTIONS as o}
            <option value={o.value} label={o.label}></option>
          {/each}
        </datalist>
      </div>
      <div class="grp">
        <label for="font-size">Font size ({appState.settings.fontSize}px)</label>
        <input
          id="font-size"
          type="range"
          min="9"
          max="22"
          step="1"
          value={appState.settings.fontSize}
          oninput={onFontSizeInput}
        />
      </div>
      <div class="actions">
        <button class="reset" onclick={resetDefaults}>Reset</button>
        <button class="close" onclick={close}>Close</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .settings { position: relative; }
  .gear {
    background:#555; color:#eee; border:0;
    padding:4px 8px; border-radius:3px; cursor:pointer;
    font-size:14px; line-height:1;
  }
  .gear:hover { background:#666; }
  .popover {
    position: absolute; top: calc(100% + 6px); right: 0;
    z-index: 100;
    background:#fff; color:#222;
    border:1px solid #bbb; border-radius:6px;
    box-shadow: 0 6px 18px rgba(0,0,0,0.25);
    padding:10px 12px;
    width: 240px;
    display:flex; flex-direction:column; gap:10px;
    font-family: var(--app-ui-font, system-ui, sans-serif);
    font-size: 12px;
  }
  .grp { display:flex; flex-direction:column; gap:4px; }
  .grp label { font-size:11px; color:#555; text-transform:uppercase; letter-spacing:.04em; }
  .grp input[type="range"], .grp input[type="text"] { width:100%; box-sizing:border-box; }
  .grp input[type="text"] { padding:3px 6px; border:1px solid #ccc; border-radius:3px; font-size:12px; font-family: inherit; }
  .hint { font-size:10px; color:#888; }
  .actions { display:flex; gap:6px; justify-content:flex-end; }
  .actions button {
    padding:4px 10px; font-size:11px; border-radius:3px; cursor:pointer;
    border:1px solid #ccc; background:#f5f5f5; color:#444;
  }
  .actions button.close { background:#2a8f3d; color:#fff; border-color:#267530; }
  .actions button:hover { filter:brightness(1.05); }
</style>
