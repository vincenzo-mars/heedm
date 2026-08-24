<script lang="ts">
import { push } from "svelte-spa-router";
import { session } from "./stores/session.svelte";
import { formatDuration, formatElapsed } from "./types";

async function handleImport() {
  if (await session.importFile()) push("/list");
}

async function handleRecord() {
  if (session.isRecording) await session.stopRecording();
  else await session.startRecording();
}
</script>

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
