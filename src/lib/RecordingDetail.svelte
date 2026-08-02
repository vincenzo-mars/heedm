<script lang="ts">
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import Button from "./Button.svelte";
import TranscriptView from "./TranscriptView.svelte";
import type { RecordingEntry } from "./types";

let {
  entry,
  onBack,
  isRecording,
  isTranscribing,
  onRetry,
}: {
  entry: RecordingEntry;
  onBack: () => void;
  isRecording: boolean;
  isTranscribing: boolean;
  onRetry: (folderPath: string) => Promise<RecordingEntry[]>;
} = $props();

// Stesso lock globale di RecordsList: mentre gira una registrazione o una
// trascrizione (rilancio incluso) non si può rilanciarne un'altra.
let locked = $derived(isRecording || isTranscribing);

async function handleRetry() {
  if (locked) return;
  // La lista aggiornata torna ad App.svelte via `onRetry`, che risincronizza
  // anche `selectedEntry`: qui non serve fare nulla col risultato.
  await onRetry(entry.folder_path);
}
</script>

<div class="flex w-full max-w-170 flex-col gap-4">
  <div class="flex items-center gap-3">
    <Button onclick={onBack} aria-label="Torna alla lista">← Lista</Button>
    <h2
      class="m-0 text-xs font-semibold tracking-wider text-brand-cream/50 uppercase"
    >
      {entry.name}
    </h2>
    <Button
      class="ml-auto"
      onclick={handleRetry}
      disabled={locked}
      title={locked ? "Attendi la fine della registrazione o della trascrizione in corso" : undefined}
    >
      Rilancia trascrizione
    </Button>
    <Button onclick={() => revealItemInDir(entry.folder_path)}>
      Apri cartella
    </Button>
  </div>

  <div class="rounded-xl border border-brand-cream/10 bg-brand-darker px-5 py-4">
    {#if entry.transcript}
      <TranscriptView transcript={entry.transcript} />
    {:else if entry.error}
      <p class="m-0 text-sm text-red-400">Trascrizione fallita: {entry.error}</p>
    {:else}
      <p class="m-0 text-sm text-brand-cream/50">
        Trascrizione non ancora disponibile.
      </p>
    {/if}
  </div>
</div>
