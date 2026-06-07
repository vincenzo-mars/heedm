<script lang="ts">
import type { TranscriptResult } from "./types";
import { formatSeconds, groupSegments, speakerColor } from "./types";

let { transcript }: { transcript: TranscriptResult } = $props();

const groups = $derived(groupSegments(transcript.segments));
</script>

<div class="flex flex-col gap-3.5">
  {#each groups as g, i (i)}
    {@const color = speakerColor(g.speaker)}
    <div class="flex flex-col gap-1">
      <div
        class="flex items-center gap-1.5 text-[0.72rem] font-bold tracking-wider uppercase"
        style={`color: ${color}`}
      >
        {g.speaker}
        <span class="font-mono font-normal opacity-60 tabular-nums">{formatSeconds(g.start)}</span>
      </div>
      <div
        class="rounded-r-lg border-l-[3px] bg-brand-darker px-3 py-2 text-sm leading-relaxed text-brand-cream/90"
        style={`border-left-color: ${color}`}
      >
        {g.text}
      </div>
    </div>
  {/each}
</div>
