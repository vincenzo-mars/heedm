<script lang="ts">
import { List } from "@lucide/svelte";
import { invoke } from "@tauri-apps/api/core";
import Button from "./lib/Button.svelte";
import RecordingDetail from "./lib/RecordingDetail.svelte";
import RecordsList from "./lib/RecordsList.svelte";
import SettingsPanel from "./lib/SettingsPanel.svelte";
import SttIndicator from "./lib/SttIndicator.svelte";
import {
  formatDuration,
  formatElapsed,
  type RecordingEntry,
  type RecordingStatus,
  type SttSettings,
  type SttStatus,
} from "./lib/types";

let isRecording = $state(false);
let durationMs = $state(0);
let error = $state<string | null>(null);
let isTranscribing = $state(false);
let transcribeMs = $state(0);
let sttStatus = $state<SttStatus>("checking");
let showSettings = $state(false);
let view = $state<"record" | "list" | "detail">("record");
let selectedEntry = $state<RecordingEntry | null>(null);

$effect(() => {
  invoke<SttSettings>("get_stt_settings").then((s) => {
    if (!s.configured) showSettings = true;
    ensureServer();
  });
});

async function ensureServer() {
  try {
    const status = await invoke<string>("check_stt_server");
    if (status === "running") {
      sttStatus = "running";
      return;
    }
    sttStatus = "starting";
    await invoke("start_stt_server");
    sttStatus = "running";
  } catch {
    sttStatus = "error";
  }
}

$effect(() => {
  if (!isTranscribing) return;
  const start = Date.now();
  transcribeMs = 0;
  const id = setInterval(() => {
    transcribeMs = Date.now() - start;
  }, 100);
  return () => clearInterval(id);
});

$effect(() => {
  if (!isRecording) return;
  const id = setInterval(async () => {
    try {
      const status = await invoke<RecordingStatus>("get_recording_status");
      durationMs = status.duration_ms;
    } catch {}
  }, 500);
  return () => clearInterval(id);
});

async function handleImport() {
  error = null;
  if (isRecording || isTranscribing) return;
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

function handleSettingsSaved(_s: SttSettings) {
  sttStatus = "checking";
  ensureServer();
}
</script>

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
        class={`h-30 w-30 cursor-pointer rounded-full border-none text-base font-bold text-brand-cream transition-[background,box-shadow,transform] duration-200 active:scale-[0.96] ${
          isRecording
            ? "animate-[pulse-rec_1.5s_ease-in-out_infinite] bg-brand-lightest shadow-[0_4px_16px_rgba(210,52,52,0.4)]"
            : "bg-brand-lighter shadow-[0_4px_16px_rgba(171,43,41,0.4)] hover:bg-brand-lightest"
        }`}
        onclick={handleRecord}
        aria-label={isRecording ? "Stop recording" : "Start recording"}
      >
        {isRecording ? "■ STOP" : "⬤ REC"}
      </button>

      {#if !isRecording}
        <button
          class="cursor-pointer border-none bg-transparent text-sm text-brand-cream/70 underline underline-offset-4 transition-colors hover:text-brand-cream disabled:cursor-default disabled:opacity-50"
          onclick={handleImport}
          disabled={isTranscribing}
        >
          o carica un file
        </button>
      {/if}

      {#if isRecording}
        <p class="m-0 font-mono text-2xl font-semibold text-brand-lightest">
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
      />
    </main>
  {:else if view === "detail" && selectedEntry}
    <main class="flex w-full flex-1 flex-col items-start gap-6 px-6 pt-6">
      <RecordingDetail
        entry={selectedEntry}
        onBack={() => {
          view = "list";
        }}
      />
    </main>
  {/if}

  <SttIndicator status={sttStatus} onSettingsClick={() => (showSettings = true)} />

  {#if showSettings}
    <SettingsPanel onClose={() => (showSettings = false)} onSaved={handleSettingsSaved} />
  {/if}
</div>
