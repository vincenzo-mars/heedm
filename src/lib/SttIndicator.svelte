<script lang="ts">
import type { SttStatus } from "./types";

let {
  status,
  onSettingsClick,
}: {
  status: SttStatus;
  onSettingsClick: () => void;
} = $props();

const labels: Record<SttStatus, string> = {
  checking: "controllo...",
  starting: "avvio server...",
  running: "server attivo",
  error: "server non disponibile",
};

const styles: Record<SttStatus, { text: string; dot: string }> = {
  checking: {
    text: "text-brand-cream/50",
    dot: "bg-gray-400 animate-[blink_1.2s_ease-in-out_infinite]",
  },
  starting: {
    text: "text-brand-cream/50",
    dot: "bg-amber-500 animate-[blink_0.8s_ease-in-out_infinite]",
  },
  running: { text: "text-green-500", dot: "bg-green-500" },
  error: { text: "text-red-400", dot: "bg-red-400" },
};
</script>

<div
  class="fixed right-4 bottom-4 z-10 flex items-center gap-2.5 rounded-full bg-brand-darker/85 py-2 pr-3 pb-2 pl-4 shadow-[0_2px_12px_rgba(0,0,0,0.3)] backdrop-blur-sm"
>
  <div class={`flex items-center gap-2 text-sm font-medium ${styles[status].text}`}>
    <span class={`h-2.5 w-2.5 shrink-0 rounded-full ${styles[status].dot}`}></span>
    {labels[status]}
  </div>
  <button
    class="border-none bg-transparent p-0.5 text-xl leading-none text-brand-cream opacity-70 transition-opacity hover:opacity-100"
    onclick={onSettingsClick}
    title="Impostazioni"
  >
    ⚙
  </button>
</div>
