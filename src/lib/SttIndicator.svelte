<script lang="ts">
import { Settings } from "@lucide/svelte";
import type { SttStatus } from "./types";

let {
  status,
  onSettingsClick,
}: {
  status: SttStatus;
  onSettingsClick: () => void;
} = $props();

const labels: Record<SttStatus, string> = {
  checking: "Controllo...",
  starting: "Avvio server...",
  running: "Server attivo",
  error: "Server non disponibile",
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
  running: { text: "text-brand-cream", dot: "bg-green-500" },
  error: { text: "text-brand-cream", dot: "bg-red-400" },
};
</script>

<div class="fixed right-4 bottom-4 z-10 flex items-center gap-2.5">
  <button
    class="rounded-full bg-brand-darker/85 p-2 shadow-[0_2px_12px_rgba(0,0,0,0.3)] hover:bg-brand-cream hover:text-brand-darker ease-in-out transition-all backdrop-blur-sm cursor-pointer"
    onclick={onSettingsClick}
    title="Impostazioni"><Settings size={16} /></button
  >
</div>

<div
  class="fixed left-4 bottom-4 py-2 px-3 z-10 flex items-center rounded-full bg-brand-darker/85 shadow-[0_2px_12px_rgba(0,0,0,0.3)]"
>
  <div
    class={`flex items-center gap-2 text-sm font-medium ${styles[status].text}`}
  >
    <span class={`h-2.5 w-2.5 shrink-0 rounded-full ${styles[status].dot}`}
    ></span>
    {labels[status]}
  </div>
</div>
