<script lang="ts">
import { List } from "@lucide/svelte";
import Button from "./lib/Button.svelte";
import Onboarding from "./lib/Onboarding.svelte";
import RecordingDetail from "./lib/RecordingDetail.svelte";
import RecordsList from "./lib/RecordsList.svelte";
import SettingsPanel from "./lib/SettingsPanel.svelte";
import SttIndicator from "./lib/SttIndicator.svelte";
import { recordings } from "./lib/stores/recordings.svelte";
import { servers } from "./lib/stores/servers.svelte";
import { session } from "./lib/stores/session.svelte";
import {
  formatDuration,
  formatElapsed,
  type RecordingEntry,
} from "./lib/types";

let showSettings = $state(false);
let view = $state<"record" | "list" | "detail">("record");
let selectedEntry = $state<RecordingEntry | null>(null);

$effect(() => {
  servers.refreshStt();
  servers.refreshLlm();
});

async function handleImport() {
  if (await session.importFile()) view = "list";
}

async function handleRecord() {
  if (session.isRecording) await session.stopRecording();
  else await session.startRecording();
}

// Il rilancio vive nella store; qui resta solo il riallineamento di
// `selectedEntry`, che è stato locale di questa vista e altrimenti resterebbe
// alla versione pre-rilancio mentre il dettaglio è aperto.
async function retryTranscription(
  folderPath: string,
): Promise<RecordingEntry[]> {
  const entries = await recordings.retryTranscription(folderPath);
  if (selectedEntry) {
    selectedEntry = recordings.byName(selectedEntry.name) ?? selectedEntry;
  }
  return entries;
}
</script>

{#if !servers.modelReady}
  <Onboarding onContinue={() => servers.refreshStt()} />
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
          session.isRecording
            ? "animate-[pulse-rec_1.5s_ease-in-out_infinite] bg-rec-strong shadow-[0_4px_16px_rgba(210,52,52,0.4)]"
            : "bg-rec shadow-[0_4px_16px_rgba(171,43,41,0.4)] hover:bg-rec-strong"
        }`}
        onclick={handleRecord}
        disabled={!session.isRecording && !session.canRecord}
        title={!session.isRecording && !session.canRecord ? session.recordGateReason : undefined}
        aria-label={session.isRecording ? "Stop recording" : "Start recording"}
      >
        {session.isRecording ? "■ STOP" : "⬤ REC"}
      </button>

      {#if !session.isRecording}
        <button
          class="cursor-pointer border-none bg-transparent text-sm text-brand-cream/70 underline underline-offset-4 transition-colors hover:text-brand-cream disabled:cursor-default disabled:opacity-50"
          onclick={handleImport}
          disabled={session.isTranscribing || !session.canRecord}
          title={!session.canRecord ? session.recordGateReason : undefined}
        >
          o carica un file
        </button>
      {/if}

      {#if !session.isRecording && !session.canRecord}
        <p class="m-0 max-w-95 text-[0.85rem] text-brand-cream/50">{session.recordGateReason}</p>
      {/if}
      {#if session.isRecording}
        <p class="m-0 font-mono text-2xl font-semibold text-rec-strong">
          {formatDuration(session.durationMs)}
        </p>
      {/if}
      {#if session.isTranscribing}
        <p class="m-0 animate-pulse text-xs text-brand-cream/50">
          Trascrizione in corso... <span class="font-mono">{formatElapsed(session.transcribeMs)}</span>
        </p>
      {/if}
      {#if session.error}
        <p class="m-0 max-w-95 text-[0.85rem] text-red-400">{session.error}</p>
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
        isRecording={session.isRecording}
        isTranscribing={session.isTranscribing}
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
        isRecording={session.isRecording}
        isTranscribing={session.isTranscribing}
        onRetry={retryTranscription}
        llmStatus={servers.llmStatus}
        llmReady={servers.llmReady}
        onLlmRefresh={servers.refreshLlm}
      />
    </main>
  {/if}

  <SttIndicator status={servers.sttStatus} onSettingsClick={() => (showSettings = true)} />

  {#if showSettings}
    <SettingsPanel
      onClose={() => (showSettings = false)}
      onSaved={() => servers.refreshStt()}
      isRecording={session.isRecording}
      isTranscribing={session.isTranscribing}
      sttStatus={servers.sttStatus}
      onServerRefresh={servers.refreshStt}
      llmStatus={servers.llmStatus}
      onLlmServerRefresh={servers.refreshLlm}
    />
  {/if}
</div>
{/if}
