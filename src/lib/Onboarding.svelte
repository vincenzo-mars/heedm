<script lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { DownloadProgress } from "./types";

let { onContinue }: { onContinue: () => void } = $props();

let downloading = $state(false);
let downloaded = $state(false);
let dlProgress = $state<DownloadProgress | null>(null);
let error = $state<string | null>(null);

$effect(() => {
  const unlisten = listen<DownloadProgress>("download-progress", (e) => {
    dlProgress = e.payload;
    if (e.payload.step === "done") {
      downloading = false;
      downloaded = true;
    }
  });

  return () => {
    unlisten.then((fn) => fn());
  };
});

async function startDownload() {
  error = null;
  downloading = true;
  dlProgress = null;
  try {
    await invoke("download_local_model");
  } catch (e) {
    downloading = false;
    error = String(e);
  }
}
</script>

<div
  class="fixed inset-0 z-[200] flex flex-col items-center justify-center gap-6 bg-brand-dark px-6 text-center"
>
  <img
    src="/icon.png"
    alt="Heedm"
    class="h-20 w-20 rounded-2xl shadow-[0_8px_24px_rgba(0,0,0,0.4)]"
  />

  <div class="flex flex-col gap-2">
    <h1 class="m-0 text-2xl font-bold text-brand-cream">Benvenuto in Heedm</h1>
    <p class="m-0 max-w-100 text-sm leading-relaxed text-brand-cream/70">
      Per trascrivere le tue registrazioni serve un modello vocale che gira
      interamente sul tuo Mac, senza inviare nulla a server esterni.
    </p>
  </div>

  {#if downloaded}
    <p class="m-0 max-w-100 text-sm leading-relaxed text-brand-cream/80">
      Modello pronto. Potrai eliminarlo o scaricarlo di nuovo in qualsiasi
      momento dalle impostazioni.
    </p>
    <button
      class="cursor-pointer rounded-lg bg-brand-cream px-6 py-2.5 text-sm font-semibold text-brand-ink transition hover:bg-brand-light"
      onclick={onContinue}
    >
      Continua
    </button>
  {:else if downloading}
    <div class="flex w-full max-w-80 flex-col gap-1.5">
      {#if dlProgress}
        <div class="h-1.5 overflow-hidden rounded-full bg-brand-darker">
          <div
            class="h-full rounded-full bg-brand-lighter transition-[width] duration-300"
            style={`width: ${dlProgress.pct}%`}
          ></div>
        </div>
        <span class="text-xs text-brand-cream/55"
          >{dlProgress.step === "model"
            ? `Modello ${dlProgress.pct}%`
            : "Completato"}</span
        >
      {/if}
    </div>
    <p class="m-0 text-xs text-brand-cream/50">
      Nel mentre puoi anche prenderti un caffè, ma non uscire dall'app!
    </p>
  {:else}
    <button
      class="cursor-pointer rounded-lg bg-brand-lighter px-6 py-2.5 text-sm font-semibold text-brand-cream transition hover:bg-brand-lightest"
      onclick={startDownload}
    >
      Scarica il modello
    </button>
    {#if error}
      <p class="m-0 max-w-100 text-[0.85rem] text-red-400">{error}</p>
    {/if}
  {/if}
</div>
