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

<div class="app">
  <main class="recorder-section">
    <h1>heedm</h1>
    <SttIndicator status={sttStatus} onSettingsClick={() => (showSettings = true)} />

    <button
      class={`rec-btn${isRecording ? " recording" : ""}`}
      onclick={handleRecord}
      aria-label={isRecording ? "Stop recording" : "Start recording"}
    >
      {isRecording ? "■ STOP" : "⬤ REC"}
    </button>

    {#if isRecording}
      <p class="timer">{formatDuration(durationMs)}</p>
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </main>

  {#if recordings.length > 0}
    <section class="recordings-list">
      <h2 class="recordings-title">Registrazioni</h2>
      {#each recordings as rec (rec.id)}
        <RecordingItem {rec} />
      {/each}
    </section>
  {/if}

  {#if showSettings}
    <SettingsPanel onClose={() => (showSettings = false)} onSaved={handleSettingsSaved} />
  {/if}
</div>
