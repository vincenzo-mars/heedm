<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { DownloadProgress, SttSettings } from "./types";

  let { onClose, onSaved }: {
    onClose: () => void;
    onSaved: (s: SttSettings) => void;
  } = $props();

  let settings = $state<SttSettings | null>(null);
  let downloading = $state(false);
  let dlProgress = $state<DownloadProgress | null>(null);
  let localReady = $state(false);

  $effect(() => {
    invoke<SttSettings>("get_stt_settings").then((s) => {
      settings = s;
      localReady = s.localReady;
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
      : null
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
          {#if downloading && dlProgress}
            <div class="dl-progress">
              <div class="dl-bar">
                <div class="dl-fill" style={`width: ${dlProgress.pct}%`}></div>
              </div>
              <span class="dl-label">{dlLabel}</span>
            </div>
          {/if}
          <button class="download-btn" onclick={startDownload} disabled={downloading}>
            {downloading ? "Download in corso..." : "Scarica"}
          </button>
        {/if}
      </div>

      <button class="save-btn" onclick={save}>
        Salva
      </button>
    </div>
  </div>
{/if}
