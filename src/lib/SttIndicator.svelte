<script lang="ts">
  import type { SttStatus } from "./types";

  let { status, onSettingsClick }: {
    status: SttStatus;
    onSettingsClick: () => void;
  } = $props();

  const labels: Record<SttStatus, string> = {
    checking: "controllo...",
    starting: "avvio server...",
    running: "server attivo",
    error: "server non disponibile",
  };
</script>

<div class="stt-row">
  <div class={`stt-indicator stt-${status}`}>
    <span class="stt-dot"></span>
    {labels[status]}
  </div>
  <button class="settings-btn" onclick={onSettingsClick} title="Impostazioni">
    ⚙
  </button>
</div>
