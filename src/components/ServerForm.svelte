<script lang="ts">
  import Icon from "./Icon.svelte";
  import Toggle from "./Toggle.svelte";
  import { api } from "../lib/api";
  import { defaultLocalKind, defaultRemoteKind, LOADERS } from "../lib/defaults";
  import { errorToast, toast } from "../lib/stores.svelte";
  import type { ServerConfig } from "../lib/types";

  let { server = $bindable(), onsave, oncancel, saving = false }: { server: ServerConfig; onsave: () => void; oncancel?: () => void; saving?: boolean } = $props();

  let detecting = $state(false);

  function setType(type: "local" | "remote") {
    if (server.kind.type === type) return;
    server.kind = type === "local" ? defaultLocalKind() : defaultRemoteKind();
  }

  async function pickDir() {
    const dir = await api.pickFolder("Dossier du serveur");
    if (dir && server.kind.type === "local") {
      server.kind.dir = dir;
      await detect();
    }
  }

  async function pickJava() {
    const f = await api.pickFile("Exécutable Java", ["exe"]);
    if (f && server.kind.type === "local") server.kind.java_path = f;
  }

  async function pickPlayit() {
    const f = await api.pickFile("Agent playit", ["exe"]);
    if (f) server.playit.exe_path = f;
  }

  async function detect() {
    if (server.kind.type !== "local" || !server.kind.dir) return;
    detecting = true;
    try {
      const d = await api.serverDetectLaunch(server.kind.dir);
      if (d.launch) server.kind.launch = d.launch;
      if (d.loader) server.loader = d.loader;
      if (d.mc_version) server.mc_version = d.mc_version;
      if (d.jvm_args) server.kind.jvm_args = d.jvm_args;
      if (d.launch) toast(`Lancement détecté : ${d.launch.mode} ${d.launch.path}`, "ok");
      for (const n of d.notes) toast(n, "warn");
    } catch (e) {
      errorToast(e);
    } finally {
      detecting = false;
    }
  }

  function setLaunchMode(mode: "jar" | "argsfile" | "script") {
    if (server.kind.type !== "local") return;
    server.kind.launch = { mode, path: server.kind.launch.path };
  }
</script>

<div class="stack">
  <div class="form-grid">
    <div class="field">
      <label for="sf-name">Nom</label>
      <input id="sf-name" class="input" bind:value={server.name} />
    </div>
    <div class="field">
      <label for="sf-type">Type</label>
      <select id="sf-type" class="input" value={server.kind.type} onchange={(e) => setType(e.currentTarget.value as "local" | "remote")}>
        <option value="local">Local (lancé par le panel)</option>
        <option value="remote">Hébergé (ping + RCON)</option>
      </select>
    </div>
    <div class="field">
      <label for="sf-loader">Loader</label>
      <select id="sf-loader" class="input" bind:value={server.loader}>
        <option value="">—</option>
        {#each LOADERS as l (l)}<option value={l}>{l}</option>{/each}
      </select>
    </div>
    <div class="field">
      <label for="sf-ver">Version Minecraft</label>
      <input id="sf-ver" class="input" bind:value={server.mc_version} placeholder="1.21.1" />
    </div>
  </div>

  {#if server.kind.type === "local"}
    <div class="form-grid">
      <div class="field full">
        <label for="sf-dir">Dossier du serveur</label>
        <div class="input-row">
          <input id="sf-dir" class="input mono" bind:value={server.kind.dir} placeholder="C:\TelekGamer\Zevent" />
          <button class="btn" onclick={pickDir}><Icon name="folder" size={14} /></button>
          <button class="btn" onclick={detect} disabled={detecting || !server.kind.dir}><Icon name="search" size={14} /> Détecter</button>
        </div>
        <span class="hint">« Détecter » repère le jar, le fichier d'arguments NeoForge/Forge, user_jvm_args.txt et la version.</span>
      </div>
      <div class="field">
        <label for="sf-mode">Mode de lancement</label>
        <select id="sf-mode" class="input" value={server.kind.launch.mode} onchange={(e) => setLaunchMode(e.currentTarget.value as "jar" | "argsfile" | "script")}>
          <option value="jar">java -jar &lt;fichier&gt;</option>
          <option value="argsfile">java @&lt;fichier d'arguments&gt; (Forge / NeoForge)</option>
          <option value="script">Script .bat (dernier recours)</option>
        </select>
      </div>
      <div class="field">
        <label for="sf-launch">{server.kind.launch.mode === "jar" ? "Jar" : server.kind.launch.mode === "argsfile" ? "Fichier d'arguments" : "Script"}</label>
        <input id="sf-launch" class="input mono" bind:value={server.kind.launch.path} placeholder={server.kind.launch.mode === "argsfile" ? "libraries/net/neoforged/neoforge/21.1.72/win_args.txt" : "server.jar"} />
        <span class="hint">Relatif au dossier du serveur.</span>
      </div>
      <div class="field">
        <label for="sf-java">Java</label>
        <div class="input-row">
          <input id="sf-java" class="input mono" bind:value={server.kind.java_path} placeholder="java" />
          <button class="btn" onclick={pickJava}><Icon name="folder" size={14} /></button>
        </div>
      </div>
      <div class="field">
        <label for="sf-port">Port</label>
        <input id="sf-port" class="input" type="number" bind:value={server.kind.port} />
      </div>
      <div class="field full">
        <label for="sf-jvm">Arguments JVM</label>
        <input id="sf-jvm" class="input mono" bind:value={server.kind.jvm_args} />
        <span class="hint">-Xmx fixe la RAM max. Les options d'encodage gardent les accents lisibles dans la console.</span>
      </div>
      <div class="field">
        <label for="sf-extra">Arguments serveur</label>
        <input id="sf-extra" class="input mono" bind:value={server.kind.extra_args} placeholder="nogui" />
      </div>
      <div class="field">
        <label for="sf-timeout">Délai avant kill après « stop » (s)</label>
        <input id="sf-timeout" class="input" type="number" bind:value={server.kind.stop_timeout_s} min="10" />
      </div>
      <div class="field full">
        <Toggle bind:checked={server.kind.auto_restart} label="Relancer automatiquement après un crash (3 fois max par 10 min)" />
      </div>
    </div>

    <h3>playit.gg</h3>
    <div class="form-grid">
      <div class="field full">
        <Toggle bind:checked={server.playit.enabled} label="Gérer l'agent playit avec ce serveur" />
      </div>
      {#if server.playit.enabled}
        <div class="field full">
          <label for="sf-playit">Exécutable playit</label>
          <div class="input-row">
            <input id="sf-playit" class="input mono" bind:value={server.playit.exe_path} placeholder="C:\Program Files\playit_gg\bin\playit.exe" />
            <button class="btn" onclick={pickPlayit}><Icon name="folder" size={14} /></button>
          </div>
        </div>
        <div class="field">
          <label for="sf-playit-args">Arguments</label>
          <input id="sf-playit-args" class="input mono" bind:value={server.playit.args} />
        </div>
        <div class="field">
          <Toggle bind:checked={server.playit.with_server} label="Démarrer et arrêter avec le serveur" />
        </div>
      {/if}
    </div>
  {:else}
    <div class="form-grid">
      <div class="field">
        <label for="sf-host">Adresse</label>
        <input id="sf-host" class="input mono" bind:value={server.kind.host} placeholder="game4.onpowered.net" />
      </div>
      <div class="field">
        <label for="sf-rport">Port de jeu</label>
        <input id="sf-rport" class="input" type="number" bind:value={server.kind.port} />
      </div>
      <div class="field full">
        <Toggle bind:checked={server.kind.rcon.enabled} label="Console via RCON" />
        <span class="hint">Dans server.properties chez l'hébergeur : enable-rcon=true, rcon.port, rcon.password. Le port doit être ouvert côté hébergeur.</span>
      </div>
      {#if server.kind.rcon.enabled}
        <div class="field">
          <label for="sf-rhost">Hôte RCON (vide = adresse du serveur)</label>
          <input id="sf-rhost" class="input mono" bind:value={server.kind.rcon.host} />
        </div>
        <div class="field">
          <label for="sf-rcport">Port RCON</label>
          <input id="sf-rcport" class="input" type="number" bind:value={server.kind.rcon.port} />
        </div>
        <div class="field full">
          <label for="sf-rpass">Mot de passe RCON</label>
          <input id="sf-rpass" class="input mono" type="password" bind:value={server.kind.rcon.password} />
        </div>
      {/if}
    </div>
  {/if}

  <div class="field">
    <label for="sf-notes">Notes</label>
    <textarea id="sf-notes" class="input" bind:value={server.notes} rows="2"></textarea>
  </div>

  <div class="row" style="justify-content: flex-end">
    {#if oncancel}<button class="btn" onclick={oncancel}>Annuler</button>{/if}
    <button class="btn primary" onclick={onsave} disabled={saving || !server.name.trim()}><Icon name="check" size={14} /> Enregistrer</button>
  </div>
</div>
