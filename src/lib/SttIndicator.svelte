<script lang="ts">
import { Settings } from "@lucide/svelte";
import Button from "./Button.svelte";
import type { SttStatus } from "./types";

let {
  status,
  onSettingsClick,
}: {
  status: SttStatus;
  onSettingsClick: () => void;
} = $props();

const STATUS: Record<SttStatus, { label: string; text: string; dot: string }> =
  {
    checking: {
      label: "Controllo...",
      text: "text-brand-cream/50",
      dot: "bg-gray-400 animate-[blink_1.2s_ease-in-out_infinite]",
    },
    starting: {
      label: "Avvio server...",
      text: "text-brand-cream/50",
      dot: "bg-amber-500 animate-[blink_0.8s_ease-in-out_infinite]",
    },
    running: {
      label: "Server attivo",
      text: "text-brand-cream",
      dot: "bg-green-500",
    },
    error: {
      label: "Server non disponibile",
      text: "text-brand-cream",
      dot: "bg-red-400",
    },
  };
</script>

<div class="fixed right-4 bottom-4 z-10 flex items-center gap-2.5">
  <Button variant="icon" onclick={onSettingsClick} title="Impostazioni">
    <Settings size={16} />
  </Button>
</div>

<div
  class="fixed left-4 bottom-4 py-2 px-3 z-10 flex items-center rounded-full bg-brand-darker/85 shadow-[0_2px_12px_rgba(0,0,0,0.3)]"
>
  <div
    class={`flex items-center gap-2 text-sm font-medium ${STATUS[status].text}`}
  >
    <span class={`h-2.5 w-2.5 shrink-0 rounded-full ${STATUS[status].dot}`}
    ></span>
    {STATUS[status].label}
  </div>
</div>
