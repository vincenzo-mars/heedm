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
let screenRecordingGranted = $state(false);

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
  refreshScreenRecordingStatus();

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
  if (!modelPath) return;
  // Il file modello potrebbe non esistere ancora (download non fatto) — rivela la cartella che lo conterrà
  const dir = modelPath.slice(0, modelPath.lastIndexOf("/"));
  revealItemInDir(dir);
}

function revealRecordings() {
  if (recordingsDir) revealItemInDir(recordingsDir);
}

async function refreshScreenRecordingStatus() {
  screenRecordingGranted = await invoke<boolean>(
    "check_screen_recording_permission",
  );
}

async function requestScreenRecording() {
  await invoke<boolean>("request_screen_recording_permission");
  await refreshScreenRecordingStatus();
}

function openPermissionSettings(pane: "microphone" | "screen-recording") {
  invoke("open_permission_settings", { pane });
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
  dlProgress?.step === "model"
    ? `Modello ${dlProgress.pct}%`
    : dlProgress?.step === "done"
      ? "Completato"
      : null,
);
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
          class="text-base leading-none text-brand-cream opacity-40 transition-opacity hover:opacity-100"
          onclick={onClose}>✕</button
        >
      </div>

      <div class="flex min-h-0 flex-col gap-4 overflow-y-auto pr-1">
        <div class="flex flex-col gap-3">
          <span class="text-[0.78rem] font-semibold text-brand-cream">Permessi</span>

          <div class="flex flex-col gap-1.5 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2.5">
            <span class="text-[0.8rem] font-medium text-brand-cream">Microfono</span>
            <p class="m-0 text-[0.74rem] leading-relaxed text-brand-cream/60">
              Necessario per registrare la tua voce.
            </p>
            <button
              class="self-start rounded-lg border border-brand-light px-3 py-1.5 text-[0.74rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
              onclick={() => openPermissionSettings("microphone")}>Apri Impostazioni di Sistema</button
            >
          </div>

          <div class="flex flex-col gap-1.5 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2.5">
            <div class="flex items-center gap-2">
              <span class={`h-2 w-2 shrink-0 rounded-full ${screenRecordingGranted ? "bg-green-500" : "bg-gray-400"}`}></span>
              <span class="text-[0.8rem] font-medium text-brand-cream">Registrazione schermo</span>
            </div>
            <p class="m-0 text-[0.74rem] leading-relaxed text-brand-cream/60">
              Necessaria per catturare l'audio di sistema (richiesta da ScreenCaptureKit).
            </p>
            <div class="flex gap-2">
              {#if !screenRecordingGranted}
                <button
                  class="rounded-lg border border-brand-light px-3 py-1.5 text-[0.74rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
                  onclick={requestScreenRecording}>Richiedi accesso</button
                >
              {/if}
              <button
                class="rounded-lg border border-brand-light px-3 py-1.5 text-[0.74rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
                onclick={() => openPermissionSettings("screen-recording")}>Apri Impostazioni di Sistema</button
              >
            </div>
          </div>
        </div>

        {#if localReady}
          <p class="m-0 text-sm font-medium text-green-400">Modello installato e pronto.</p>
        {:else}
          <p class="m-0 text-[0.82rem] leading-relaxed text-brand-cream/80">
            Scarica il modello Whisper large-v3-turbo (~1.5 GB).
            Necessario solo al primo avvio.
          </p>
        {/if}

        <div class="flex flex-col gap-1.5">
          <span class="text-[0.78rem] font-semibold text-brand-cream">Cartella modello</span>
          {#if modelPath}
            <div class="flex flex-col gap-1 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2">
              <span class="text-[0.62rem] font-semibold tracking-wider text-brand-cream/40 uppercase"
                >Percorso</span
              >
              <p class="m-0 font-mono text-[0.72rem] break-all text-brand-cream/70">{modelPath}</p>
            </div>
          {/if}
          <div class="flex gap-2">
            <button
              class="flex-1 rounded-lg border border-brand-light px-4 py-2.5 text-[0.8rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
              onclick={changeModelDir}>Cambia cartella</button
            >
            {#if modelPath}
              <button
                class="flex-1 rounded-lg border border-brand-light px-4 py-2.5 text-[0.8rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
                onclick={revealModel}>Mostra nel Finder</button
              >
            {/if}
          </div>

          {#if downloading && dlProgress}
            <div class="flex flex-col gap-1 pt-1">
              <div class="h-1.5 overflow-hidden rounded-full bg-brand-dark">
                <div
                  class="h-full rounded-full bg-brand-lighter transition-[width] duration-300"
                  style={`width: ${dlProgress.pct}%`}
                ></div>
              </div>
              <span class="text-xs text-brand-cream/55">{dlLabel}</span>
            </div>
          {/if}

          <button
            class="rounded-lg bg-brand-lighter px-4 py-2.5 text-[0.85rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
            onclick={startDownload}
            disabled={downloading}
          >
            {downloading
              ? "Download in corso..."
              : localReady
                ? "Scarica di nuovo"
                : "Scarica"}
          </button>
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="text-[0.78rem] font-semibold text-brand-cream">Cartella registrazioni</span>
          {#if recordingsDir}
            <div class="flex flex-col gap-1 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2">
              <span class="text-[0.62rem] font-semibold tracking-wider text-brand-cream/40 uppercase"
                >Percorso</span
              >
              <p class="m-0 font-mono text-[0.72rem] break-all text-brand-cream/70">
                {recordingsDir}
              </p>
            </div>
          {/if}
          <div class="flex gap-2">
            <button
              class="flex-1 rounded-lg border border-brand-light px-4 py-2.5 text-[0.8rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
              onclick={changeRecordingsDir}>Cambia cartella</button
            >
            {#if recordingsDir}
              <button
                class="flex-1 rounded-lg border border-brand-light px-4 py-2.5 text-[0.8rem] font-semibold text-brand-light transition hover:bg-brand-light/10"
                onclick={revealRecordings}>Mostra nel Finder</button
              >
            {/if}
          </div>
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
