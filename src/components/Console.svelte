<script lang="ts">
  import { onMount, tick } from "svelte";
  import Icon from "./Icon.svelte";
  import { api, on } from "../lib/api";
  import { complete, hasContinuation, type Suggestion } from "../lib/commands";
  import { app, errorToast } from "../lib/stores.svelte";
  import type { LogLine } from "../lib/types";

  let { id, canSend, players }: { id: string; canSend: boolean; players: string[] } = $props();

  const LINE_H = 20;
  const OVERSCAN = 12;

  let lines = $state<LogLine[]>([]);
  let filterLevel = $state<"all" | "warn" | "error">("all");
  let search = $state("");
  let stick = $state(true);
  let scrollTop = $state(0);
  let viewportH = $state(400);
  let container: HTMLDivElement | undefined = $state();
  let input = $state("");
  let inputEl: HTMLInputElement | undefined = $state();
  let selected = $state(0);
  let history = $state<string[]>([]);
  let historyPos = $state(-1);
  let showSuggestions = $state(false);

  const filtered = $derived.by(() => {
    const q = search.trim().toLowerCase();
    return lines.filter((l) => {
      if (filterLevel === "error" && !(l.level === "error" || l.level === "fatal")) return false;
      if (filterLevel === "warn" && !(l.level === "warn" || l.level === "error" || l.level === "fatal")) return false;
      if (q && !l.raw.toLowerCase().includes(q)) return false;
      return true;
    });
  });

  const start = $derived(Math.max(0, Math.floor(scrollTop / LINE_H) - OVERSCAN));
  const end = $derived(Math.min(filtered.length, Math.ceil((scrollTop + viewportH) / LINE_H) + OVERSCAN));
  const visible = $derived(filtered.slice(start, end));

  const completion = $derived(complete(input, { players }));
  const suggestions = $derived<Suggestion[]>(showSuggestions && input.trim().length > 0 ? completion.suggestions.slice(0, 12) : []);

  onMount(() => {
    let unlisten: (() => void) | undefined;
    (async () => {
      try {
        lines = await api.serverLog(id);
      } catch (e) {
        errorToast(e);
      }
      await tick();
      scrollToBottom();
      unlisten = await on<{ server_id: string; lines: LogLine[] }>("server:log", (batch) => {
        if (batch.server_id !== id) return;
        lines.push(...batch.lines);
        const cap = app.config?.settings.log_buffer_lines ?? 2000;
        if (lines.length > cap) lines.splice(0, lines.length - cap);
      });
    })();
    try {
      history = JSON.parse(localStorage.getItem(`history:${id}`) ?? "[]");
    } catch {
      history = [];
    }
    return () => unlisten?.();
  });

  $effect(() => {
    void filtered.length;
    if (stick) tick().then(scrollToBottom);
  });

  function scrollToBottom() {
    if (container) container.scrollTop = container.scrollHeight;
  }

  function onscroll(e: Event) {
    const el = e.currentTarget as HTMLDivElement;
    scrollTop = el.scrollTop;
    stick = el.scrollHeight - el.scrollTop - el.clientHeight < LINE_H * 2;
  }

  function lineClass(l: LogLine): string {
    if (l.logger === "panel") return "panel";
    if (l.logger === "console") return "echo";
    if (l.logger === "rcon") return "rcon";
    if (l.logger === "playit") return "playit";
    if (l.level === "error" || l.level === "fatal") return "error";
    if (l.level === "warn") return "warn";
    if (l.continuation) return "cont";
    return "";
  }

  function accept(s: Suggestion) {
    const before = input.slice(0, completion.wordStart);
    const next = `${before}${s.value}`;
    input = hasContinuation(next) ? `${next} ` : next;
    selected = 0;
    showSuggestions = true;
    inputEl?.focus();
  }

  async function send() {
    const cmd = input.trim();
    if (!cmd || !canSend) return;
    input = "";
    showSuggestions = false;
    historyPos = -1;
    history = [cmd, ...history.filter((h) => h !== cmd)].slice(0, 100);
    try {
      localStorage.setItem(`history:${id}`, JSON.stringify(history));
    } catch {
      // ignoré
    }
    try {
      await api.serverCommand(id, cmd);
    } catch (e) {
      errorToast(e);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Tab" && suggestions.length) {
      e.preventDefault();
      accept(suggestions[selected] ?? suggestions[0]);
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (suggestions.length && selected > 0) {
        accept(suggestions[selected]);
        return;
      }
      send();
      return;
    }
    if (e.key === "Escape") {
      showSuggestions = false;
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (suggestions.length) {
        selected = (selected - 1 + suggestions.length) % suggestions.length;
      } else if (history.length) {
        historyPos = Math.min(historyPos + 1, history.length - 1);
        input = history[historyPos];
        showSuggestions = false;
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (suggestions.length) {
        selected = (selected + 1) % suggestions.length;
      } else if (historyPos >= 0) {
        historyPos -= 1;
        input = historyPos >= 0 ? history[historyPos] : "";
      }
      return;
    }
    showSuggestions = true;
  }

  async function clear() {
    lines = [];
    try {
      await api.serverClearLog(id);
    } catch (e) {
      errorToast(e);
    }
  }
</script>

<div class="console">
  <div class="toolbar">
    <div class="levels">
      <button class:active={filterLevel === "all"} onclick={() => (filterLevel = "all")}>Tout</button>
      <button class:active={filterLevel === "warn"} onclick={() => (filterLevel = "warn")}>Avertissements</button>
      <button class:active={filterLevel === "error"} onclick={() => (filterLevel = "error")}>Erreurs</button>
    </div>
    <div class="search">
      <Icon name="search" size={14} />
      <input class="input" placeholder="Filtrer les logs" bind:value={search} />
    </div>
    <span class="muted small">{filtered.length} lignes</span>
    <button class="btn ghost sm" onclick={clear} title="Vider l'affichage"><Icon name="trash" size={14} /></button>
  </div>

  <div class="log" bind:this={container} bind:clientHeight={viewportH} {onscroll}>
    {#if filtered.length === 0}
      <div class="empty-log muted">Aucune ligne. La console se remplit dès que le serveur écrit.</div>
    {:else}
      <div class="spacer" style:height="{filtered.length * LINE_H}px">
        <div class="window" style:transform="translateY({start * LINE_H}px)">
          {#each visible as l, i (start + i)}
            <div class="line {lineClass(l)}" style:height="{LINE_H}px">
              <span class="ts">{l.ts}</span>
              <span class="msg">{l.continuation ? l.raw : l.message}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  {#if !stick && filtered.length > 0}
    <button class="btn sm resume" onclick={() => { stick = true; scrollToBottom(); }}><Icon name="chevron-down" size={14} /> Reprendre le défilement</button>
  {/if}

  <div class="input-wrap">
    {#if suggestions.length}
      <div class="suggestions">
        {#each suggestions as s, i (s.value)}
          <button class:selected={i === selected} onmousedown={(e) => { e.preventDefault(); accept(s); }}>
            <span class="mono">{s.value}</span>
            {#if s.desc}<span class="desc">{s.desc}</span>{/if}
          </button>
        {/each}
        {#if completion.hint}<div class="hint mono">{completion.hint}</div>{/if}
      </div>
    {:else if input.trim() && completion.hint}
      <div class="suggestions"><div class="hint mono">{completion.hint}</div></div>
    {/if}
    <div class="cmd">
      <span class="prompt mono">/</span>
      <input
        class="input mono"
        bind:this={inputEl}
        bind:value={input}
        {onkeydown}
        onfocus={() => (showSuggestions = true)}
        onblur={() => (showSuggestions = false)}
        placeholder={canSend ? "Commande (Tab pour compléter, ↑↓ historique)" : "Serveur hors ligne"}
        disabled={!canSend}
        spellcheck="false"
        autocomplete="off"
      />
      <button class="btn primary" onclick={send} disabled={!canSend || !input.trim()}><Icon name="send" size={14} /> Envoyer</button>
    </div>
  </div>
</div>

<style>
  .console {
    display: flex;
    flex-direction: column;
    height: calc(100vh - 250px);
    min-height: 420px;
    background: var(--bg-elev);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    position: relative;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .levels {
    display: flex;
    gap: 2px;
    background: var(--bg-elev);
    border-radius: var(--radius-sm);
    padding: 2px;
  }

  .levels button {
    border: 0;
    background: transparent;
    color: var(--text-2);
    padding: 4px 9px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 12.5px;
  }

  .levels button.active {
    background: var(--surface-3);
    color: var(--text);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 6px;
    flex: 1;
    max-width: 360px;
    color: var(--muted);
  }

  .search .input {
    padding: 4px 8px;
  }

  .log {
    flex: 1;
    overflow: auto;
    font-family: var(--mono);
    font-size: 12.5px;
    padding: 6px 0;
  }

  .spacer {
    position: relative;
  }

  .window {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
  }

  .line {
    display: flex;
    gap: 10px;
    padding: 0 12px;
    white-space: pre;
    line-height: 20px;
    color: var(--text-2);
  }

  .line .ts {
    color: var(--muted);
    flex: 0 0 60px;
  }

  .line .msg {
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .line.warn {
    color: var(--warn);
  }

  .line.error {
    color: var(--err);
  }

  .line.cont {
    color: var(--muted);
  }

  .line.panel {
    color: var(--accent);
  }

  .line.echo {
    color: var(--blue);
  }

  .line.rcon {
    color: var(--text);
  }

  .line.playit {
    color: #c9a3ff;
  }

  .empty-log {
    padding: 24px;
    font-family: var(--font);
  }

  .resume {
    position: absolute;
    right: 18px;
    bottom: 64px;
    box-shadow: var(--shadow);
  }

  .input-wrap {
    position: relative;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }

  .cmd {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
  }

  .prompt {
    color: var(--muted);
  }

  .cmd .input {
    flex: 1;
  }

  .suggestions {
    position: absolute;
    left: 28px;
    bottom: 100%;
    width: 440px;
    max-width: calc(100% - 40px);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    margin-bottom: 4px;
    overflow: hidden;
  }

  .suggestions button {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    width: 100%;
    text-align: left;
    border: 0;
    background: transparent;
    color: var(--text);
    padding: 5px 10px;
    cursor: pointer;
  }

  .suggestions button.selected,
  .suggestions button:hover {
    background: var(--surface-3);
  }

  .suggestions .desc {
    color: var(--muted);
    font-size: 12px;
  }

  .suggestions .hint {
    padding: 5px 10px;
    font-size: 11.5px;
    color: var(--muted);
    border-top: 1px solid var(--border);
    background: var(--surface);
  }
</style>
