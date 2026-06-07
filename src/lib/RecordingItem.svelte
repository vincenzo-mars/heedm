<script lang="ts">
import TranscriptView from "./TranscriptView.svelte";
import type { Recording } from "./types";

let { rec }: { rec: Recording } = $props();
</script>

<div class="flex flex-col gap-3 rounded-xl border border-brand-cream/10 bg-brand-darker px-5 py-4">
  <div class="flex items-center gap-2.5">
    <span class="flex-1 truncate text-sm font-semibold text-brand-cream">{rec.filename}</span>
    {#if rec.status === "transcribing"}
      <span
        class="flex items-center gap-1.25 rounded-full border border-brand-lighter/30 bg-brand-lighter/15 px-2 py-0.75 text-[0.7rem] font-semibold whitespace-nowrap text-brand-light"
      >
        <span class="flex items-center gap-0.5">
          <span
            class="h-1 w-1 rounded-full bg-brand-light animate-[blink_1.2s_ease-in-out_infinite]"
          ></span>
          <span
            class="h-1 w-1 rounded-full bg-brand-light animate-[blink_1.2s_ease-in-out_infinite] [animation-delay:0.2s]"
          ></span>
          <span
            class="h-1 w-1 rounded-full bg-brand-light animate-[blink_1.2s_ease-in-out_infinite] [animation-delay:0.4s]"
          ></span>
        </span>
        trascrizione
      </span>
    {/if}
    {#if rec.status === "done"}
      <span
        class="rounded-full border border-green-800/60 bg-green-950/40 px-2 py-0.75 text-[0.7rem] font-semibold whitespace-nowrap text-green-400"
        >fatto</span
      >
    {/if}
    {#if rec.status === "error"}
      <span
        class="rounded-full border border-red-800/60 bg-red-950/40 px-2 py-0.75 text-[0.7rem] font-semibold whitespace-nowrap text-red-400"
        >errore</span
      >
    {/if}
  </div>

  {#if rec.status === "transcribing"}
    <div class="flex flex-col gap-2">
      <div
        class="h-3 rounded-md bg-linear-to-r from-white/5 via-white/15 to-white/5 bg-size-[200%_100%] animate-[shimmer_1.4s_ease-in-out_infinite]"
        style="width: 80%"
      ></div>
      <div
        class="h-3 rounded-md bg-linear-to-r from-white/5 via-white/15 to-white/5 bg-size-[200%_100%] animate-[shimmer_1.4s_ease-in-out_infinite]"
        style="width: 55%"
      ></div>
      <div
        class="h-3 rounded-md bg-linear-to-r from-white/5 via-white/15 to-white/5 bg-size-[200%_100%] animate-[shimmer_1.4s_ease-in-out_infinite]"
        style="width: 70%"
      ></div>
    </div>
  {/if}
  {#if rec.status === "error"}
    <p class="m-0 max-w-95 text-[0.85rem] text-red-400">{rec.error}</p>
  {/if}
  {#if rec.status === "done" && rec.transcript}
    <TranscriptView transcript={rec.transcript} />
  {/if}
</div>
