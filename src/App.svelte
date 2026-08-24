<script lang="ts">
import { List } from "@lucide/svelte";
import { invoke } from "@tauri-apps/api/core";
import Button from "./lib/Button.svelte";
import Onboarding from "./lib/Onboarding.svelte";
import RecordingDetail from "./lib/RecordingDetail.svelte";
import RecordsList from "./lib/RecordsList.svelte";
import SettingsPanel from "./lib/SettingsPanel.svelte";
import SttIndicator from "./lib/SttIndicator.svelte";
import {
  formatDuration,
  formatElapsed,
  type RecordingEntry,
  type ServerStatus,
  type SttSettings,
  type SttStatus,
} from "./lib/types";

let isRecording = $state(false);
let durationMs = $state(0);
let error = $state<string | null>(null);
let isTranscribing = $state(false);
let transcribeMs = $state(0);
let sttStatus = $state<SttStatus>("checking");
let modelReady = $state(false);
let llmStatus = $state<ServerStatus>("checking");
let llmReady = $state(false);
let showSettings = $state(false);
let view = $state<"record" | "list" | "detail">("record");
let selectedEntry = $state<RecordingEntry | null>(null);

// Gate per registrazione e import: senza server whisper attivo e modello sul
// disco, `transcribe_recording` non può funzionare. Lo stesso controllo è
// applicato anche lato backend (start_recording/import_audio_file), qui serve
// solo a riflettersi in UI.
let canRecord = $derived(sttStatus === "running" && modelReady);
let recordGateReason = $derived.by(() => {
  if (canRecord) return null;
  if (!modelReady)
    return "Scarica il modello dalle impostazioni per registrare.";
  return "Avvia il server whisper dalle impostazioni per registrare.";
});

$effect(() => {
  refreshSttState();
  refreshLlmState();
});

// Punto unico di riallineamento fra stato reale (whisper-server + modello sul
// disco) e stato mostrato in UI. Chiamata da qui al mount e da SettingsPanel
// dopo un salvataggio; i task futuri (onboarding post-download, stop/restart/
// delete dalle impostazioni, rilancio trascrizione) devono passare da qui
// invece di duplicare la logica di check/avvio.
// `attemptStart: false` serve a chi ha appena fermato il server di proposito
// (T5): senza, il check troverebbe "stopped" e lo riavvierebbe subito.
async function refreshSttState(
  opts: { attemptStart?: boolean } = {},
): Promise<SttSettings | null> {
  const attemptStart = opts.attemptStart ?? true;
  sttStatus = "checking";
  try {
    const settings = await invoke<SttSettings>("get_stt_settings");
    modelReady = settings.localReady;
    const status = await invoke<string>("check_stt_server");
    if (status === "running") {
      sttStatus = "running";
      return settings;
    }
    if (!attemptStart) {
      sttStatus = "stopped";
      return settings;
    }
    if (!modelReady) {
      // Senza modello whisper-server fallirebbe comunque all'avvio: risparmia
      // il giro e riflette subito lo stato reale.
      sttStatus = "error";
      return settings;
    }
    sttStatus = "starting";
    await invoke("start_stt_server");
    sttStatus = "running";
    return settings;
  } catch {
    sttStatus = "error";
    return null;
  }
}

// Mirror di refreshSttState per il server LLM (riassunto/chat), con un
// default invertito: `attemptStart` è `false` qui (STT ce l'ha `true`).
// Al mount osserviamo soltanto: non ha senso caricare 1-5GB di modello per
// una feature che molte sessioni non toccano mai. Diventa `true` solo
// quando l'utente preme esplicitamente "Avvia" (Settings o pannello note).
async function refreshLlmState(
  opts: { attemptStart?: boolean } = {},
): Promise<SttSettings | null> {
  const attemptStart = opts.attemptStart ?? false;
  llmStatus = "checking";
  try {
    const settings = await invoke<SttSettings>("get_stt_settings");
    llmReady = settings.llmReady;
    const status = await invoke<string>("check_llm_server");
    if (status === "running") {
      llmStatus = "running";
      return settings;
    }
    if (status === "loading") {
      llmStatus = "loading";
      return settings;
    }
    if (!attemptStart) {
      llmStatus = "stopped";
      return settings;
    }
    llmStatus = "starting";
    await invoke("start_llm_server");
    llmStatus = "loading";
    return settings;
  } catch {
    llmStatus = "error";
    return null;
  }
}

// llama-server, a differenza di whisper-server, può restare "loading" a
// lungo (download del modello al primo avvio): questo poll periodico è
// l'unico modo per la UI di accorgersi quando diventa "running" da solo,
// senza un secondo click dell'utente.
$effect(() => {
  if (llmStatus !== "loading" && llmStatus !== "starting") return;
  const id = setInterval(async () => {
    try {
      const status = await invoke<string>("check_llm_server");
      if (status === "running") llmStatus = "running";
      else if (status === "stopped") llmStatus = "stopped";
      else llmStatus = "loading";
    } catch {
      llmStatus = "error";
    }
  }, 2000);
  return () => clearInterval(id);
});

$effect(() => {
  if (!isTranscribing) return;
  const start = Date.now();
  transcribeMs = 0;
  const id = setInterval(() => {
    transcribeMs = Date.now() - start;
  }, 100);
  return () => clearInterval(id);
});

// Timer locale, stesso pattern di transcribeMs: il frontend è l'unico a poter
// avviare la registrazione, quindi l'inizio lo conosce già senza chiedere al
// backend via IPC ogni mezzo secondo.
$effect(() => {
  if (!isRecording) return;
  const start = Date.now();
  const id = setInterval(() => {
    durationMs = Date.now() - start;
  }, 500);
  return () => clearInterval(id);
});

async function handleImport() {
  error = null;
  if (isRecording || isTranscribing) return;
  if (!canRecord) {
    error = recordGateReason;
    return;
  }
  try {
    const path = await invoke<string | null>("import_audio_file");
    if (!path) return;
    isTranscribing = true;
    try {
      await invoke("transcribe_recording", { path });
      view = "list";
    } finally {
      isTranscribing = false;
    }
  } catch (e) {
    error = String(e);
  }
}

async function handleRecord() {
  error = null;
  if (!isRecording) {
    if (!canRecord) {
      error = recordGateReason;
      return;
    }
    try {
      await invoke("start_recording");
      isRecording = true;
      durationMs = 0;
    } catch (e) {
      error = String(e);
    }
  } else {
    try {
      const path = await invoke<string>("stop_recording");
      isRecording = false;
      if (!path) return;
      isTranscribing = true;
      try {
        await invoke("transcribe_recording", { path });
      } finally {
        isTranscribing = false;
      }
    } catch (e) {
      isRecording = false;
      error = String(e);
    }
  }
}

// Rilancio trascrizione da RecordsList/RecordingDetail: riusa lo stesso lock
// `isTranscribing` del flusso REC/import (blocca REC e le altre righe), poi
// ricarica `list_recordings` come unica fonte di verità (niente stato locale
// scollegato dal disco) e, se il record rilanciato è quello aperto in
// dettaglio, riallinea `selectedEntry` così `RecordingDetail` non resta stale.
async function retryTranscription(
  folderPath: string,
): Promise<RecordingEntry[]> {
  if (isRecording || isTranscribing) return [];
  isTranscribing = true;
  try {
    await invoke("transcribe_recording", {
      path: `${folderPath}/recording.wav`,
    });
  } catch {
    // Esito già persistito su disco da transcribe_recording (transcript.json
    // o transcript_error.json): list_recordings sotto lo riflette da solo.
  } finally {
    isTranscribing = false;
  }
  const entries = await invoke<RecordingEntry[]>("list_recordings");
  if (selectedEntry) {
    selectedEntry =
      entries.find((e) => e.folder_path === selectedEntry?.folder_path) ??
      selectedEntry;
  }
  return entries;
}
</script>

{#if !modelReady}
  <Onboarding onContinue={() => refreshSttState()} />
{:else}
<div
  class="flex h-screen flex-col items-center gap-8 overflow-y-auto px-6 pt-6 pb-18 box-border"
>
  <Button
    variant="icon"
    class="fixed right-4 top-4 z-10"
    onclick={() => { view = "list"; }}
    title="Registrazioni"
    aria-label="Vai alle registrazioni"
  >
    <List size={16} />
  </Button>

  {#if view === "record"}
    <main
      class="flex flex-1 flex-col items-center justify-center gap-6 text-center"
    >
      <button
        class={`h-30 w-30 cursor-pointer rounded-full border-none text-base font-bold text-brand-cream transition-[background,box-shadow,transform] duration-200 active:scale-[0.96] disabled:cursor-default disabled:opacity-50 ${
          isRecording
            ? "animate-[pulse-rec_1.5s_ease-in-out_infinite] bg-rec-strong shadow-[0_4px_16px_rgba(210,52,52,0.4)]"
            : "bg-rec shadow-[0_4px_16px_rgba(171,43,41,0.4)] hover:bg-rec-strong"
        }`}
        onclick={handleRecord}
        disabled={!isRecording && !canRecord}
        title={!isRecording && !canRecord ? recordGateReason : undefined}
        aria-label={isRecording ? "Stop recording" : "Start recording"}
      >
        {isRecording ? "■ STOP" : "⬤ REC"}
      </button>

      {#if !isRecording}
        <button
          class="cursor-pointer border-none bg-transparent text-sm text-brand-cream/70 underline underline-offset-4 transition-colors hover:text-brand-cream disabled:cursor-default disabled:opacity-50"
          onclick={handleImport}
          disabled={isTranscribing || !canRecord}
          title={!canRecord ? recordGateReason : undefined}
        >
          o carica un file
        </button>
      {/if}

      {#if !isRecording && !canRecord}
        <p class="m-0 max-w-95 text-[0.85rem] text-brand-cream/50">{recordGateReason}</p>
      {/if}
      {#if isRecording}
        <p class="m-0 font-mono text-2xl font-semibold text-rec-strong">
          {formatDuration(durationMs)}
        </p>
      {/if}
      {#if isTranscribing}
        <p class="m-0 animate-pulse text-xs text-brand-cream/50">
          Trascrizione in corso... <span class="font-mono">{formatElapsed(transcribeMs)}</span>
        </p>
      {/if}
      {#if error}
        <p class="m-0 max-w-95 text-[0.85rem] text-red-400">{error}</p>
      {/if}
    </main>
  {:else if view === "list"}
    <main class="flex w-full flex-1 flex-col items-start gap-6 px-6 pt-6">
      <RecordsList
        onSelect={(e) => {
          selectedEntry = e;
          view = "detail";
        }}
        onBack={() => {
          view = "record";
        }}
        {isRecording}
        {isTranscribing}
        onRetry={retryTranscription}
      />
    </main>
  {:else if view === "detail" && selectedEntry}
    <main class="flex w-full flex-1 flex-col items-start gap-6 px-6 pt-6">
      <RecordingDetail
        entry={selectedEntry}
        onBack={() => {
          view = "list";
        }}
        {isRecording}
        {isTranscribing}
        onRetry={retryTranscription}
        {llmStatus}
        {llmReady}
        onLlmRefresh={refreshLlmState}
      />
    </main>
  {/if}

  <SttIndicator status={sttStatus} onSettingsClick={() => (showSettings = true)} />

  {#if showSettings}
    <SettingsPanel
      onClose={() => (showSettings = false)}
      onSaved={() => refreshSttState()}
      {isRecording}
      {isTranscribing}
      {sttStatus}
      onServerRefresh={refreshSttState}
      {llmStatus}
      onLlmServerRefresh={refreshLlmState}
    />
  {/if}
</div>
{/if}
