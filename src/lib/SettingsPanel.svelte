<script lang="ts">
import { Folder, Mic, MonitorCheck, MonitorX, X } from "@lucide/svelte";
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
let modelPath = $state<string | null>(null);
let recordingsDir = $state<string | null>(null);
let screenRecordingGranted = $state(false);

$effect(() => {
  invoke<SttSettings>("get_stt_settings").then((s) => {
    settings = s;
  });
  invoke<string>("get_local_model_path").then((p) => {
    modelPath = p;
  });
  invoke<string>("get_recordings_dir").then((d) => {
    recordingsDir = d;
  });
  refreshScreenRecordingStatus();

  const unlisten = listen<DownloadProgress>("download-progress", (e) => {
    dlProgress = e.payload;
    if (e.payload.step === "done") {
      downloading = false;
      if (settings) settings = { ...settings, localReady: true };
    }
  });

  return () => {
    unlisten.then((fn) => fn());
  };
});

async function save() {
  if (!settings) return;
  const updated: SttSettings = { ...settings, configured: true };
  await invoke("save_stt_settings", { settings: updated });
  onSaved(updated);
  onClose();
}

function revealRecordings() {
  if (recordingsDir) revealItemInDir(recordingsDir);
}

async function refreshScreenRecordingStatus() {
  screenRecordingGranted = await invoke<boolean>(
    "check_screen_recording_permission",
  );
}

function openPermissionSettings(pane: "microphone" | "screen-recording") {
  invoke("open_permission_settings", { pane });
}
async function startDownload() {
  downloading = true;
  dlProgress = null;
  try {
    await invoke("download_local_model");
  } catch (e) {
    downloading = false;
    throw e;
  }
}
</script>

{#if settings}
  <div
    class="fixed inset-0 z-[100] flex items-center justify-center bg-black/50"
    role="presentation"
    onclick={onClose}
  >
    <div
      class="flex max-h-[85vh] w-[min(420px,90vw)] flex-col gap-5 rounded-2xl bg-brand-darker p-6 text-brand-cream shadow-[0_20px_60px_rgba(0,0,0,0.5)]"
      role="presentation"
      onclick={(e) => e.stopPropagation()}
    >
      <div class="flex items-center justify-between">
        <span class="text-base font-bold">Impostazioni</span>
        <button
          class="text-base leading-none text-brand-cream opacity-40 transition-opacity hover:opacity-100 cursor-pointer"
          onclick={onClose}><X size={20} /></button
        >
      </div>

      <div class="flex min-h-0 flex-col gap-4 overflow-y-auto pr-1">
        <div class="flex flex-col gap-3">
          <span class="text-[0.78rem] font-semibold text-brand-cream"
            >Permessi</span
          >

          <div class="flex flex-col">
            <button
              type="button"
              class="flex items-start gap-2.5 rounded-lg px-2 py-1.5 text-left transition hover:bg-brand-dark/50 cursor-pointer"
              onclick={() => openPermissionSettings("microphone")}
            >
              <Mic size={16} class="mt-0.5 shrink-0 text-brand-cream/70" />
              <div class="flex flex-col gap-0.5">
                <span class="text-[0.8rem] font-medium text-brand-cream"
                  >Microfono</span
                >
                <p
                  class="m-0 text-[0.7rem] leading-relaxed text-brand-cream/60"
                >
                  Necessario per registrare la tua voce.
                </p>
              </div>
            </button>

            <button
              type="button"
              class="flex items-start gap-2.5 rounded-lg px-2 py-1.5 text-left transition hover:bg-brand-dark/50 cursor-pointer"
              onclick={() => openPermissionSettings("screen-recording")}
            >
              {#if screenRecordingGranted}
                <MonitorCheck
                  size={16}
                  class="mt-0.5 shrink-0 text-green-500"
                />
              {:else}
                <MonitorX
                  size={16}
                  class="mt-0.5 shrink-0 text-brand-cream/70"
                />
              {/if}
              <div class="flex flex-col gap-0.5">
                <span class="text-[0.8rem] font-medium text-brand-cream"
                  >Cattura audio sistema</span
                >
                <p
                  class="m-0 text-[0.7rem] leading-relaxed text-brand-cream/60"
                >
                  Necessaria per catturare l'audio di sistema (richiesta da
                  ScreenCaptureKit).
                </p>
              </div>
            </button>
          </div>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between">
            <span class="text-[0.78rem] font-semibold text-brand-cream"
              >Modello locale</span
            >

            {#if modelPath}
              <button
                class="p-2 cursor-pointer font-semibold text-brand-light transition hover:text-brand-cream"
                onclick={() => revealItemInDir(modelPath!.slice(0, modelPath!.lastIndexOf("/")))}><Folder size={20} /></button
              >
            {/if}
          </div>

          {#if settings?.localReady}
            <p class="m-0 text-xs font-medium text-green-400">
              Modello installato e pronto.
            </p>
          {:else}
            <p class="m-0 text-[0.82rem] leading-relaxed text-brand-cream/80">
              Scarica il modello Whisper large-v3-turbo (~1.5 GB). Necessario
              solo al primo avvio.
            </p>
          {/if}

          {#if modelPath}
            <div
              class="flex flex-col gap-1 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2"
            >
              <span
                class="text-[0.62rem] font-semibold tracking-wider text-brand-cream/40 uppercase"
                >Percorso</span
              >
              <p
                class="m-0 font-mono text-[0.72rem] break-all text-brand-cream/70"
              >
                {modelPath}
              </p>
            </div>
          {/if}

          {#if downloading && dlProgress}
            <div class="flex flex-col gap-1 pt-1">
              <div class="h-1.5 overflow-hidden rounded-full bg-brand-dark">
                <div
                  class="h-full rounded-full bg-brand-lighter transition-[width] duration-300"
                  style={`width: ${dlProgress.pct}%`}
                ></div>
              </div>
              <span class="text-xs text-brand-cream/55">{dlProgress?.step === "model" ? `Modello ${dlProgress.pct}%` : "Completato"}</span>
            </div>
          {/if}

          <button
            class="rounded-lg bg-brand-lighter px-4 py-2.5 text-[0.85rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
            onclick={startDownload}
            disabled={downloading}
          >
            {downloading
              ? "Download in corso..."
              : settings?.localReady
                ? "Scarica di nuovo"
                : "Scarica"}
          </button>
        </div>

        <div class="flex flex-col gap-1.5">
          <div class="flex items-center justify-between">
            <span class="text-[0.78rem] font-semibold text-brand-cream"
              >Cartella registrazioni</span
            >

            {#if recordingsDir}
              <button
                class="p-2 cursor-pointer font-semibold text-brand-light transition hover:text-brand-cream"
                onclick={revealRecordings}><Folder size={20} /></button
              >
            {/if}
          </div>

          {#if recordingsDir}
            <div
              class="flex flex-col gap-1 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2"
            >
              <span
                class="text-[0.62rem] font-semibold tracking-wider text-brand-cream/40 uppercase"
                >Percorso</span
              >
              <p
                class="m-0 font-mono text-[0.72rem] break-all text-brand-cream/70"
              >
                {recordingsDir}
              </p>
            </div>
          {/if}
        </div>
      </div>

      <button
        class="rounded-lg bg-brand-cream p-2.5 text-sm font-semibold text-brand-ink transition hover:bg-brand-light"
        onclick={save}
      >
        Salva
      </button>
    </div>
  </div>
{/if}
