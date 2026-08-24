<script lang="ts">
import { List } from "@lucide/svelte";
import Router, { push } from "svelte-spa-router";
import Button from "./lib/Button.svelte";
import Onboarding from "./lib/Onboarding.svelte";
import { routes } from "./lib/routes";
import SttIndicator from "./lib/SttIndicator.svelte";
import { servers } from "./lib/stores/servers.svelte";

$effect(() => {
  servers.refreshStt();
  servers.refreshLlm();
});
</script>

{#if !servers.modelReady}
  <Onboarding onContinue={() => servers.refreshStt()} />
{:else}
<div
  class="flex h-screen flex-col items-center gap-8 overflow-y-auto px-6 pt-6 pb-18 box-border"
>
  <Button
    variant="icon"
    class="fixed right-4 top-4 z-10"
    onclick={() => push("/list")}
    title="Registrazioni"
    aria-label="Vai alle registrazioni"
  >
    <List size={16} />
  </Button>

  <Router {routes} />

  <SttIndicator status={servers.sttStatus} />
</div>
{/if}
