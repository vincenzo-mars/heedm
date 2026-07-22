<script lang="ts">
import { invoke } from "@tauri-apps/api/core";
import Button from "./Button.svelte";
import { formatElapsed, type RecordingEntry } from "./types";

let {
  onSelect,
  onBack,
}: {
  onSelect: (entry: RecordingEntry) => void;
  onBack: () => void;
} = $props();

let entries = $state<RecordingEntry[]>([]);
let loading = $state(true);

$effect(() => {
  invoke<RecordingEntry[]>("list_recordings")
    .then((r) => {
      entries = r;
    })
    .finally(() => {
      loading = false;
    });
});
</script>

<div class="flex w-full max-w-170 flex-col gap-4">
  <div class="flex items-center gap-3">
    <Button onclick={onBack} aria-label="Torna indietro">← Indietro</Button>
    <h2
      class="m-0 text-xs font-semibold tracking-wider text-brand-cream/50 uppercase"
    >
      Registrazioni
    </h2>
  </div>

  {#if loading}
    <p class="text-sm text-brand-cream/50">Caricamento...</p>
  {:else if entries.length === 0}
    <p class="text-sm text-brand-cream/50">Nessuna registrazione trovata.</p>
  {:else}
    {#each entries as entry (entry.folder_path)}
      <button
        class="flex cursor-pointer items-center justify-between rounded-xl border border-brand-cream/10 bg-brand-darker px-5 py-4 text-left transition-colors hover:border-brand-cream/20"
        onclick={() => onSelect(entry)}
      >
        <span class="text-sm font-semibold text-brand-cream">{entry.name}</span>
        <span class="flex items-center gap-2">
          {#if entry.transcript?.transcription_ms != null}
            <span class="whitespace-nowrap font-mono text-[0.7rem] text-brand-cream/40">
              {formatElapsed(entry.transcript.transcription_ms)}
            </span>
          {/if}
          <span
            class={`whitespace-nowrap rounded-full px-2 py-0.5 text-[0.7rem] font-semibold ${
              entry.transcript
                ? "border border-green-800/60 bg-green-950/40 text-green-400"
                : "border border-brand-cream/20 bg-transparent text-brand-cream/40"
            }`}
          >
            {entry.transcript ? "trascritto" : "in attesa"}
          </span>
        </span>
      </button>
    {/each}
  {/if}
</div>
