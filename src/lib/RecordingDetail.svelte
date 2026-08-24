<script lang="ts">
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { push, replace } from "svelte-spa-router";
import Button from "./Button.svelte";
import { recordings } from "./stores/recordings.svelte";
import { servers } from "./stores/servers.svelte";
import { session } from "./stores/session.svelte";
import TranscriptNotes from "./TranscriptNotes.svelte";
import TranscriptView from "./TranscriptView.svelte";

let { params }: { params: { id: string } } = $props();

let id = $derived(decodeURIComponent(params.id));
let entry = $derived(recordings.byName(id));
let resolving = $state(true);

// La store può essere vuota se si arriva qui direttamente dall'URL (reload
// con l'hash già su /detail, o HMR in dev): finché `load()` non ha risposto
// non si può concludere che la registrazione non esista. Dopo, un id senza
// riscontro è una registrazione cancellata e si torna alla lista con
// `replace`, per non lasciare in history una route morta.
$effect(() => {
  const wanted = id;
  resolving = true;
  recordings.load().then(() => {
    if (wanted !== id) return;
    resolving = false;
    if (!recordings.byName(wanted)) replace("/list");
  });
});

async function handleRetry() {
  if (session.locked || !entry) return;
  await recordings.retryTranscription(entry.folder_path);
}
</script>

<div class="flex w-full max-w-170 flex-col gap-4">
  {#if !entry}
    <p class="text-sm text-brand-cream/50">
      {resolving ? "Caricamento..." : "Registrazione non trovata."}
    </p>
  {:else}
    <div class="flex items-center gap-3">
      <Button onclick={() => push("/list")} aria-label="Torna alla lista">← Lista</Button>
      <h2
        class="m-0 text-xs font-semibold tracking-wider text-brand-cream/50 uppercase"
      >
        {entry.name}
      </h2>
      <Button
        class="ml-auto"
        onclick={handleRetry}
        disabled={session.locked}
        title={session.locked ? "Attendi la fine della registrazione o della trascrizione in corso" : undefined}
      >
        Rilancia trascrizione
      </Button>
      <Button onclick={() => entry && revealItemInDir(entry.folder_path)}>
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

    <TranscriptNotes
      {entry}
      llmStatus={servers.llmStatus}
      llmReady={servers.llmReady}
      onLlmRefresh={servers.refreshLlm}
    />
  {/if}
</div>
