<script lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type { DownloadProgress, SttSettings } from "./types";

let {
  onClose,
  onSaved,
}: {
  onClose: () => void;
  onSaved: (s: SttSettings) => void;
} = $props();

let settings = $state<SttSettings | null>(null);
let downloading = $state(false);
let dlProgress = $state<DownloadProgress | null>(null);
let localReady = $state(false);
let modelPath = $state<string | null>(null);
let recordingsDir = $state<string | null>(null);

$effect(() => {
  invoke<SttSettings>("get_stt_settings").then((s) => {
    settings = s;
    localReady = s.localReady;
  });
  invoke<string>("get_local_model_path").then((p) => {
    modelPath = p;
  });
  invoke<string>("get_recordings_dir").then((d) => {
    recordingsDir = d;
  });

  const unlisten = listen<DownloadProgress>("download-progress", (e) => {
    dlProgress = e.payload;
    if (e.payload.step === "done") {
      downloading = false;
      localReady = true;
    }
  });

  return () => {
    unlisten.then((fn) => fn());
  };
});

async function save() {
  if (!settings) return;
  const updated: SttSettings = {
    ...settings,
    localReady,
    configured: true,
  };
  await invoke("save_stt_settings", { settings: updated });
  onSaved(updated);
  onClose();
}

function revealModel() {
  if (modelPath) revealItemInDir(modelPath);
}

function revealRecordings() {
  if (recordingsDir) revealItemInDir(recordingsDir);
}

async function changeModelDir() {
  if (!settings) return;
  const dir = await invoke<string | null>("pick_directory");
  if (!dir) return;
  if (
    !confirm(
      "Cambiando cartella dovrai riscaricare il modello nella nuova posizione. Continuare?",
    )
  ) {
    return;
  }
  settings = { ...settings, modelDir: dir, localReady: false };
  localReady = false;
  await invoke("save_stt_settings", { settings });
  modelPath = await invoke<string>("get_local_model_path");
}

async function changeRecordingsDir() {
  if (!settings) return;
  const dir = await invoke<string | null>("pick_directory");
  if (!dir) return;
  settings = { ...settings, recordingsDir: dir };
  await invoke("save_stt_settings", { settings });
  recordingsDir = dir;
}

async function startDownload() {
  downloading = true;
  dlProgress = null;
  try {
    await invoke("download_local_model");
  } catch (e) {
    downloading = false;
    alert(String(e));
  }
}

const dlLabel = $derived(
  dlProgress?.step === "binary"
    ? `Binary ${dlProgress.pct}%`
    : dlProgress?.step === "model"
      ? `Modello ${dlProgress.pct}%`
      : dlProgress?.step === "done"
        ? "Completato"
        : null,
);
</script>

{#if settings}
  <div class="settings-backdrop" role="presentation" onclick={onClose}>
    <div class="settings-panel" role="presentation" onclick={(e) => e.stopPropagation()}>
      <div class="settings-header">
        <span class="settings-title">Trascrizione</span>
        <button class="settings-close" onclick={onClose}>✕</button>
      </div>

      <div class="mode-content">
        {#if localReady}
          <p class="local-ready">Modello installato e pronto.</p>
        {:else}
          <p class="local-warning">
            Scarica whisper-server + modello large-v3-turbo (~1.5 GB).
            Necessario solo al primo avvio.
          </p>
        {/if}

        <div class="path-row">
          <span class="path-label">Cartella modello</span>
          {#if modelPath}
            <p class="model-path">{modelPath}</p>
          {/if}
          <button class="path-btn" onclick={changeModelDir}>Cambia cartella</button>
        </div>

        <div class="path-row">
          <span class="path-label">Cartella registrazioni</span>
          {#if recordingsDir}
            <p class="model-path">{recordingsDir}</p>
          {/if}
          <div class="path-actions">
            <button class="path-btn" onclick={changeRecordingsDir}>Cambia cartella</button>
            {#if recordingsDir}
              <button class="reveal-btn" onclick={revealRecordings}>Mostra nel Finder</button>
            {/if}
          </div>
        </div>

        {#if downloading && dlProgress}
          <div class="dl-progress">
            <div class="dl-bar">
              <div class="dl-fill" style={`width: ${dlProgress.pct}%`}></div>
            </div>
            <span class="dl-label">{dlLabel}</span>
          </div>
        {/if}

        <div class="model-actions">
          <button class="download-btn" onclick={startDownload} disabled={downloading}>
            {downloading
              ? "Download in corso..."
              : localReady
                ? "Scarica di nuovo"
                : "Scarica"}
          </button>
          {#if localReady}
            <button class="reveal-btn" onclick={revealModel}>Mostra nel Finder</button>
          {/if}
        </div>
      </div>

      <button class="save-btn" onclick={save}>
        Salva
      </button>
    </div>
  </div>
{/if}
