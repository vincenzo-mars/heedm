<script lang="ts">
import { push } from "svelte-spa-router";
import Button from "./Button.svelte";
import { recordings } from "./stores/recordings.svelte";
import { session } from "./stores/session.svelte";
import {
  formatElapsed,
  type RecordingEntry,
  transcriptStatusInfo,
} from "./types";

$effect(() => {
  recordings.load();
});

async function handleRetry(entry: RecordingEntry) {
  if (session.locked) return;
  await recordings.retryTranscription(entry.folder_path);
}
</script>

<div class="flex w-full max-w-170 flex-col gap-4">
  <div class="flex items-center gap-3">
    <Button onclick={() => push("/")} aria-label="Torna indietro">← Indietro</Button>
    <h2
      class="m-0 text-xs font-semibold tracking-wider text-brand-cream/50 uppercase"
    >
      Registrazioni
    </h2>
  </div>

  {#if recordings.loading && recordings.all.length === 0}
    <p class="text-sm text-brand-cream/50">Caricamento...</p>
  {:else if recordings.all.length === 0}
    <p class="text-sm text-brand-cream/50">Nessuna registrazione trovata.</p>
  {:else}
    {#each recordings.all as entry (entry.folder_path)}
      {@const status = transcriptStatusInfo(entry)}
      <div
        class="flex items-center justify-between gap-3 rounded-xl border border-brand-cream/10 bg-brand-darker px-5 py-4 transition-colors hover:border-brand-cream/20"
      >
        <button
          class="flex flex-1 cursor-pointer items-center justify-between gap-3 border-none bg-transparent p-0 text-left disabled:cursor-default"
          onclick={() => push(`/detail/${encodeURIComponent(entry.name)}`)}
          disabled={session.locked}
        >
          <span class="text-sm font-semibold text-brand-cream">{entry.name}</span>
          <span class="flex items-center gap-2">
            {#if entry.transcript?.transcription_ms != null}
              <span class="whitespace-nowrap font-mono text-[0.7rem] text-brand-cream/40">
                {formatElapsed(entry.transcript.transcription_ms)}
              </span>
            {/if}
            <span
              class={`whitespace-nowrap rounded-full px-2 py-0.5 text-[0.7rem] font-semibold ${status.className}`}
            >
              {status.label}
            </span>
          </span>
        </button>
        <Button
          onclick={() => handleRetry(entry)}
          disabled={session.locked}
          title={session.locked ? "Attendi la fine della registrazione o della trascrizione in corso" : undefined}
        >
          Rilancia
        </Button>
      </div>
    {/each}
  {/if}
</div>
