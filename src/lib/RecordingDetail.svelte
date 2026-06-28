<script lang="ts">
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import TranscriptView from "./TranscriptView.svelte";
import type { RecordingEntry } from "./types";

let {
  entry,
  onBack,
}: {
  entry: RecordingEntry;
  onBack: () => void;
} = $props();
</script>

<div class="flex w-full max-w-170 flex-col gap-4">
  <div class="flex items-center gap-3">
    <button
      class="cursor-pointer text-brand-cream/60 transition-colors hover:text-brand-cream"
      onclick={onBack}
      aria-label="Torna alla lista"
    >
      ← Lista
    </button>
    <h2
      class="m-0 text-xs font-semibold tracking-wider text-brand-cream/50 uppercase"
    >
      {entry.name}
    </h2>
    <button
      class="ml-auto cursor-pointer rounded-lg border border-brand-cream/15 bg-brand-darker px-3 py-1.5 text-xs text-brand-cream/70 transition-colors hover:text-brand-cream"
      onclick={() => revealItemInDir(entry.folder_path)}
    >
      Apri cartella
    </button>
  </div>

  <div class="rounded-xl border border-brand-cream/10 bg-brand-darker px-5 py-4">
    {#if entry.transcript}
      <TranscriptView transcript={entry.transcript} />
    {:else}
      <p class="m-0 text-sm text-brand-cream/50">
        Trascrizione non ancora disponibile.
      </p>
    {/if}
  </div>
</div>
