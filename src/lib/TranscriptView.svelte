<script lang="ts">
import type { TranscriptResult } from "./types";
import { formatSeconds, groupSegments, speakerColor } from "./types";

let { transcript }: { transcript: TranscriptResult } = $props();

const groups = $derived(groupSegments(transcript.segments));
</script>

<div class="transcript">
  {#each groups as g, i (i)}
    {@const color = speakerColor(g.speaker)}
    <div class="speaker-block">
      <div class="speaker-label" style={`color: ${color}`}>
        {g.speaker}
        <span class="speaker-time">{formatSeconds(g.start)}</span>
      </div>
      <div class="speaker-bubble" style={`border-left-color: ${color}`}>
        {g.text}
      </div>
    </div>
  {/each}
</div>
