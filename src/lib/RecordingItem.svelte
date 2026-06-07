<script lang="ts">
import TranscriptView from "./TranscriptView.svelte";
import type { Recording } from "./types";

let { rec }: { rec: Recording } = $props();
</script>

<div class="recording-item">
  <div class="recording-header">
    <span class="recording-filename">{rec.filename}</span>
    {#if rec.status === "transcribing"}
      <span class="badge badge-transcribing">
        <span class="dots"><span></span><span></span><span></span></span>
        trascrizione
      </span>
    {/if}
    {#if rec.status === "done"}
      <span class="badge badge-done">fatto</span>
    {/if}
    {#if rec.status === "error"}
      <span class="badge badge-error">errore</span>
    {/if}
  </div>

  {#if rec.status === "transcribing"}
    <div class="skeleton-lines">
      <div class="skeleton" style="width: 80%"></div>
      <div class="skeleton" style="width: 55%"></div>
      <div class="skeleton" style="width: 70%"></div>
    </div>
  {/if}
  {#if rec.status === "error"}
    <p class="error">{rec.error}</p>
  {/if}
  {#if rec.status === "done" && rec.transcript}
    <TranscriptView transcript={rec.transcript} />
  {/if}
</div>
