<script lang="ts">
import { Folder, Mic, MonitorCheck, MonitorX, X } from "@lucide/svelte";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import type {
  DownloadProgress,
  HfGgufFile,
  HfModelSummary,
  LlmDownloadProgress,
  ServerStatus,
  SttSettings,
  SttStatus,
} from "./types";

let {
  onClose,
  onSaved,
  isRecording = false,
  isTranscribing = false,
  sttStatus,
  onServerRefresh,
  llmStatus,
  onLlmServerRefresh,
}: {
  onClose: () => void;
  onSaved: (s: SttSettings) => void;
  isRecording?: boolean;
  isTranscribing?: boolean;
  sttStatus: SttStatus;
  onServerRefresh: (opts?: {
    attemptStart?: boolean;
  }) => Promise<SttSettings | null>;
  llmStatus: ServerStatus;
  onLlmServerRefresh: (opts?: {
    attemptStart?: boolean;
  }) => Promise<SttSettings | null>;
} = $props();

let settings = $state<SttSettings | null>(null);
let downloading = $state(false);
let dlProgress = $state<DownloadProgress | null>(null);
let llmDownloading = $state(false);
let llmDlProgress = $state<LlmDownloadProgress | null>(null);
let modelPath = $state<string | null>(null);
let recordingsDir = $state<string | null>(null);
let screenRecordingGranted = $state(false);
let sttLoading = $state(false);
let sttError = $state<string | null>(null);
let llmLoading = $state(false);
let llmError = $state<string | null>(null);

let systemRamGb = $state<number | null>(null);
let hfQuery = $state("");
let hfSearching = $state(false);
let hfResults = $state<HfModelSummary[]>([]);
let expandedRepo = $state<string | null>(null);
let hfFilesLoading = $state(false);
let hfFilesGated = $state(false);
let hfFiles = $state<HfGgufFile[]>([]);
let searchDebounce: ReturnType<typeof setTimeout> | undefined;

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
  invoke<number>("get_system_memory_gb").then((gb) => {
    systemRamGb = gb;
  });
  runSearch("instruct");

  const unlisten = listen<DownloadProgress>("download-progress", (e) => {
    dlProgress = e.payload;
    if (e.payload.step === "done") {
      downloading = false;
      if (settings) settings = { ...settings, localReady: true };
    }
  });

  const unlistenLlm = listen<LlmDownloadProgress>(
    "llm-download-progress",
    async (e) => {
      llmDlProgress = e.payload;
      if (e.payload.step === "done") {
        llmDownloading = false;
        // Ricarica da disco invece di settare llmReady in ottimismo: la
        // verità è sempre il file, stesso motivo per cui get_stt_settings
        // la ricalcola lato Rust invece di fidarsi del valore persistito.
        settings = await invoke<SttSettings>("get_stt_settings");
        await onLlmServerRefresh({ attemptStart: false });
      }
    },
  );

  return () => {
    unlisten.then((fn) => fn());
    unlistenLlm.then((fn) => fn());
  };
});

async function save() {
  if (!settings) return;
  const updated: SttSettings = { ...settings, configured: true };
  await invoke("save_stt_settings", { settings: updated });
  onSaved(updated);
  onClose();
}

async function handleStartServer() {
  sttLoading = true;
  sttError = null;
  try {
    await invoke("start_stt_server");
    await onServerRefresh();
  } catch (e) {
    sttError = String(e);
  } finally {
    sttLoading = false;
  }
}

async function handleRestartServer() {
  sttLoading = true;
  sttError = null;
  try {
    await invoke("restart_stt_server");
    await onServerRefresh();
  } catch (e) {
    sttError = String(e);
  } finally {
    sttLoading = false;
  }
}

async function handleStopServer() {
  sttLoading = true;
  sttError = null;
  try {
    await invoke("stop_stt_server");
    await onServerRefresh({ attemptStart: false });
  } catch (e) {
    sttError = String(e);
  } finally {
    sttLoading = false;
  }
}

async function handleDeleteModel() {
  if (!settings) return;
  sttLoading = true;
  sttError = null;
  try {
    await invoke("delete_local_model");
    await onServerRefresh();
    onSaved(settings);
    onClose();
  } catch (e) {
    sttError = String(e);
  } finally {
    sttLoading = false;
  }
}

// ── Server LLM ────────────────────────────────────────────────────────────────

async function handleStartLlmServer() {
  llmLoading = true;
  llmError = null;
  try {
    await invoke("start_llm_server");
    await onLlmServerRefresh({ attemptStart: false });
  } catch (e) {
    llmError = String(e);
  } finally {
    llmLoading = false;
  }
}

async function handleRestartLlmServer() {
  llmLoading = true;
  llmError = null;
  try {
    await invoke("restart_llm_server");
    await onLlmServerRefresh({ attemptStart: false });
  } catch (e) {
    llmError = String(e);
  } finally {
    llmLoading = false;
  }
}

async function handleStopLlmServer() {
  llmLoading = true;
  llmError = null;
  try {
    await invoke("stop_llm_server");
    await onLlmServerRefresh({ attemptStart: false });
  } catch (e) {
    llmError = String(e);
  } finally {
    llmLoading = false;
  }
}

async function handleClearLlmCache() {
  llmLoading = true;
  llmError = null;
  try {
    await invoke("clear_llm_cache");
    await onLlmServerRefresh({ attemptStart: false });
  } catch (e) {
    llmError = String(e);
  } finally {
    llmLoading = false;
  }
}

// ── Ricerca modelli Hugging Face ────────────────────────────────────────────────

function scheduleSearch() {
  clearTimeout(searchDebounce);
  searchDebounce = setTimeout(
    () => runSearch(hfQuery.trim() || "instruct"),
    400,
  );
}

async function runSearch(query: string) {
  hfSearching = true;
  llmError = null;
  try {
    hfResults = await invoke<HfModelSummary[]>("search_hf_models", { query });
  } catch (e) {
    llmError = String(e);
  } finally {
    hfSearching = false;
  }
}

async function toggleRepo(id: string) {
  if (expandedRepo === id) {
    expandedRepo = null;
    return;
  }
  expandedRepo = id;
  hfFiles = [];
  hfFilesGated = false;
  hfFilesLoading = true;
  try {
    const detail = await invoke<{
      gated: boolean;
      context_length: number | null;
      files: HfGgufFile[];
    }>("get_hf_model_files", { repoId: id });
    hfFilesGated = detail.gated;
    hfFiles = [...detail.files].sort((a, b) => a.size_bytes - b.size_bytes);
  } catch (e) {
    llmError = String(e);
  } finally {
    hfFilesLoading = false;
  }
}

async function selectModel(repo: string, file: HfGgufFile) {
  if (!settings) return;
  await invoke("set_llm_model", {
    repo,
    file: file.filename,
    sizeBytes: file.size_bytes,
  });
  settings = {
    ...settings,
    llmHfRepo: repo,
    llmHfFile: file.filename,
    llmSizeBytes: file.size_bytes,
    llmReady: false,
  };
}

async function startLlmDownload() {
  llmDownloading = true;
  llmDlProgress = null;
  try {
    await invoke("download_llm_model");
  } catch (e) {
    llmDownloading = false;
    llmError = String(e);
  }
}

function formatGb(bytes: number): string {
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}

// Euristica, mai bloccante: solo un suggerimento visivo, l'utente può
// comunque scegliere un file "pesante" se la sua macchina ce la fa.
function ramBadge(
  sizeBytes: number,
): { label: string; className: string } | null {
  if (systemRamGb == null) return null;
  const ratio = sizeBytes / 1024 ** 3 / systemRamGb;
  if (ratio < 0.35)
    return { label: "Consigliato per la tua RAM", className: "text-green-400" };
  if (ratio < 0.6)
    return { label: "Ok per la tua RAM", className: "text-amber-400" };
  return { label: "Pesante per la tua RAM", className: "text-red-400" };
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
      class="flex max-h-[88vh] w-[min(1400px,92vw)] flex-col gap-5 rounded-2xl bg-brand-darker p-6 text-brand-cream shadow-[0_20px_60px_rgba(0,0,0,0.5)]"
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

      <div
        class="grid min-h-0 grid-cols-1 gap-x-8 gap-y-5 overflow-y-auto pr-1 lg:grid-cols-2 lg:items-start"
      >
      <div class="flex flex-col gap-5">
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
              >Modello Whisper</span
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
          <span class="text-[0.78rem] font-semibold text-brand-cream"
            >Server STT</span
          >

          <div class="flex flex-col gap-2 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2">
            <div class="flex items-center justify-between">
              <span class="text-[0.8rem] text-brand-cream/70">Stato:</span>
              <span class={`text-[0.8rem] font-medium ${
                sttStatus === "running" ? "text-green-400" : "text-brand-cream/60"
              }`}>
                {sttStatus === "running" ? "Attivo" : "Fermo"}
              </span>
            </div>

            {#if sttError}
              <p class="m-0 text-[0.75rem] text-red-400 leading-tight">
                {sttError}
              </p>
            {/if}
          </div>

          <div class="grid grid-cols-2 gap-1.5">
            <button
              class="rounded-lg bg-brand-lighter px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={handleStartServer}
              disabled={sttLoading || isRecording || isTranscribing}
            >
              {sttLoading ? "..." : "Avvia"}
            </button>

            <button
              class="rounded-lg bg-brand-lighter px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={handleRestartServer}
              disabled={sttLoading || isRecording || isTranscribing}
            >
              {sttLoading ? "..." : "Riavvia"}
            </button>

            <button
              class="rounded-lg bg-brand-lighter px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={handleStopServer}
              disabled={sttLoading || isRecording || isTranscribing}
            >
              {sttLoading ? "..." : "Spegni"}
            </button>

            <button
              class="rounded-lg bg-red-600 px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-red-700 disabled:cursor-not-allowed disabled:bg-red-600/30"
              onclick={handleDeleteModel}
              disabled={sttLoading || isRecording || isTranscribing}
            >
              {sttLoading ? "..." : "Elimina modello"}
            </button>
          </div>
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

      <div class="flex flex-col gap-5">
        <div class="flex flex-col gap-1.5">
          <span class="text-[0.78rem] font-semibold text-brand-cream">Modello LLM</span>
          <p class="m-0 text-[0.72rem] leading-relaxed text-brand-cream/60">
            Per il riassunto e la chat sulle trascrizioni. Opzionale: nulla si blocca se non lo scarichi.
          </p>

          {#if systemRamGb != null}
            <p class="m-0 text-[0.68rem] text-brand-cream/40">RAM rilevata: {systemRamGb.toFixed(1)} GB</p>
          {/if}

          {#if settings?.llmHfRepo && settings?.llmHfFile}
            <div class="flex flex-col gap-1 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2">
              <span class="text-[0.62rem] font-semibold tracking-wider text-brand-cream/40 uppercase">Selezionato</span>
              <p class="m-0 font-mono text-[0.72rem] break-all text-brand-cream/70">
                {settings.llmHfRepo} · {settings.llmHfFile}
              </p>
            </div>

            {#if llmDownloading && llmDlProgress}
              <div class="flex flex-col gap-1 pt-1">
                <div class="h-1.5 overflow-hidden rounded-full bg-brand-dark">
                  <div
                    class="h-full rounded-full bg-brand-lighter transition-[width] duration-300"
                    style={`width: ${llmDlProgress.pct}%`}
                  ></div>
                </div>
                <span class="text-xs text-brand-cream/55">{llmDlProgress?.step === "llm" ? `Modello ${llmDlProgress.pct}%` : "Completato"}</span>
              </div>
            {/if}

            <button
              class="rounded-lg bg-brand-lighter px-4 py-2.5 text-[0.85rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={startLlmDownload}
              disabled={llmDownloading}
            >
              {llmDownloading
                ? "Download in corso..."
                : settings?.llmReady
                  ? `Scarica di nuovo (${formatGb(settings.llmSizeBytes)})`
                  : `Scarica modello (${formatGb(settings.llmSizeBytes)})`}
            </button>
          {/if}

          <input
            type="text"
            class="rounded-lg border border-brand-cream/15 bg-brand-dark/50 px-2.5 py-1.5 text-[0.8rem] text-brand-cream placeholder:text-brand-cream/40 focus:outline-none"
            placeholder="Cerca un modello GGUF su Hugging Face..."
            bind:value={hfQuery}
            oninput={scheduleSearch}
          />

          <div class="flex max-h-[46vh] flex-col gap-1 overflow-y-auto">
            {#if hfSearching}
              <p class="m-0 text-[0.75rem] text-brand-cream/50">Ricerca in corso...</p>
            {:else if hfResults.length === 0}
              <p class="m-0 text-[0.75rem] text-brand-cream/50">Nessun risultato.</p>
            {:else}
              {#each hfResults as result (result.id)}
                <div class="rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-1.5">
                  <button
                    type="button"
                    class="flex w-full cursor-pointer items-center justify-between gap-2 border-none bg-transparent p-0 text-left"
                    onclick={() => toggleRepo(result.id)}
                  >
                    <span class="truncate text-[0.78rem] font-medium text-brand-cream">{result.id}</span>
                    <span class="shrink-0 text-[0.65rem] text-brand-cream/40">
                      {result.license ?? "?"} · {result.downloads.toLocaleString()} dl
                    </span>
                  </button>

                  {#if expandedRepo === result.id}
                    <div class="mt-1.5 flex flex-col gap-1 border-t border-brand-cream/10 pt-1.5">
                      {#if hfFilesLoading}
                        <p class="m-0 text-[0.72rem] text-brand-cream/50">Carico i file...</p>
                      {:else if hfFilesGated}
                        <p class="m-0 text-[0.72rem] text-amber-400">
                          🔒 Richiede autenticazione Hugging Face: non supportato.
                        </p>
                      {:else if hfFiles.length === 0}
                        <p class="m-0 text-[0.72rem] text-brand-cream/50">Nessun file GGUF trovato.</p>
                      {:else}
                        {#each hfFiles as file (file.filename)}
                          {@const badge = ramBadge(file.size_bytes)}
                          <button
                            type="button"
                            class="flex cursor-pointer items-center justify-between gap-2 rounded-md border-none bg-transparent px-1.5 py-1 text-left transition hover:bg-brand-cream/5"
                            onclick={() => selectModel(result.id, file)}
                          >
                            <span class="truncate text-[0.72rem] text-brand-cream/80">{file.filename}</span>
                            <span class="shrink-0 text-right text-[0.65rem]">
                              <span class="text-brand-cream/50">{formatGb(file.size_bytes)}</span>
                              {#if badge}
                                <span class={`ml-1 ${badge.className}`}>· {badge.label}</span>
                              {/if}
                            </span>
                          </button>
                        {/each}
                      {/if}
                    </div>
                  {/if}
                </div>
              {/each}
            {/if}
          </div>
        </div>

        <div class="flex flex-col gap-1.5">
          <span class="text-[0.78rem] font-semibold text-brand-cream">Server LLM</span>

          <div class="flex flex-col gap-2 rounded-lg border border-brand-cream/10 bg-brand-dark/50 px-2.5 py-2">
            <div class="flex items-center justify-between">
              <span class="text-[0.8rem] text-brand-cream/70">Stato:</span>
              <span class={`text-[0.8rem] font-medium ${
                llmStatus === "running" ? "text-green-400" : "text-brand-cream/60"
              }`}>
                {llmStatus === "running" ? "Attivo" : llmStatus === "loading" ? "Caricamento..." : "Fermo"}
              </span>
            </div>

            {#if llmError}
              <p class="m-0 text-[0.75rem] text-red-400 leading-tight">
                {llmError}
              </p>
            {:else if !settings?.llmReady}
              <p class="m-0 text-[0.75rem] text-brand-cream/50 leading-tight">
                Scarica prima il modello.
              </p>
            {/if}
          </div>

          <div class="grid grid-cols-2 gap-1.5">
            <button
              class="rounded-lg bg-brand-lighter px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={handleStartLlmServer}
              disabled={llmLoading || isRecording || isTranscribing || !settings?.llmReady}
            >
              {llmLoading ? "..." : "Avvia"}
            </button>

            <button
              class="rounded-lg bg-brand-lighter px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={handleRestartLlmServer}
              disabled={llmLoading || isRecording || isTranscribing || !settings?.llmReady}
            >
              {llmLoading ? "..." : "Riavvia"}
            </button>

            <button
              class="rounded-lg bg-brand-lighter px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-brand-lightest disabled:cursor-not-allowed disabled:bg-brand-light/30"
              onclick={handleStopLlmServer}
              disabled={llmLoading || isRecording || isTranscribing}
            >
              {llmLoading ? "..." : "Spegni"}
            </button>

            <button
              class="rounded-lg bg-red-600 px-3 py-2 text-[0.8rem] font-semibold text-brand-cream transition hover:bg-red-700 disabled:cursor-not-allowed disabled:bg-red-600/30"
              onclick={handleClearLlmCache}
              disabled={llmLoading || isRecording || isTranscribing}
            >
              {llmLoading ? "..." : "Elimina modelli scaricati"}
            </button>
          </div>
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
