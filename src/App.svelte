<script lang="ts">
import { invoke } from "@tauri-apps/api/core";
import RecordingItem from "./lib/RecordingItem.svelte";
import SettingsPanel from "./lib/SettingsPanel.svelte";
import SttIndicator from "./lib/SttIndicator.svelte";
import {
  formatDuration,
  type Recording,
  type RecordingStatus,
  type SttSettings,
  type SttStatus,
  type TranscriptResult,
} from "./lib/types";

let isRecording = $state(false);
let durationMs = $state(0);
let error = $state<string | null>(null);
let recordings = $state<Recording[]>([]);
let sttStatus = $state<SttStatus>("checking");
let showSettings = $state(false);

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
  if (!isRecording) return;
  const id = setInterval(async () => {
    try {
      const status = await invoke<RecordingStatus>("get_recording_status");
      durationMs = status.duration_ms;
    } catch {}
  }, 500);
  return () => clearInterval(id);
});

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

      const filename = path.split("/").pop() ?? path;
      const id = crypto.randomUUID();

      recordings = [
        { id, path, filename, status: "transcribing" },
        ...recordings,
      ];

      try {
        const transcript = await invoke<TranscriptResult>(
          "transcribe_recording",
          { path },
        );
        recordings = recordings.map((r) =>
          r.id === id ? { ...r, status: "done", transcript } : r,
        );
      } catch (e) {
        recordings = recordings.map((r) =>
          r.id === id ? { ...r, status: "error", error: String(e) } : r,
        );
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

<div class="flex h-screen flex-col items-center gap-8 overflow-y-auto px-6 pt-6 pb-18 box-border">
  <main class="flex flex-1 flex-col items-center justify-center gap-6 text-center">
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

    {#if isRecording}
      <p class="m-0 font-mono text-2xl font-semibold text-brand-lightest">
        {formatDuration(durationMs)}
      </p>
    {/if}
    {#if error}
      <p class="m-0 max-w-95 text-[0.85rem] text-red-400">{error}</p>
    {/if}
  </main>

  {#if recordings.length > 0}
    <section class="flex w-full max-w-170 flex-col gap-4">
      <h2 class="m-0 mb-1 text-xs font-semibold tracking-wider text-brand-cream/50 uppercase">
        Registrazioni
      </h2>
      {#each recordings as rec (rec.id)}
        <RecordingItem {rec} />
      {/each}
    </section>
  {/if}

  <SttIndicator status={sttStatus} onSettingsClick={() => (showSettings = true)} />

  {#if showSettings}
    <SettingsPanel onClose={() => (showSettings = false)} onSaved={handleSettingsSaved} />
  {/if}
</div>
