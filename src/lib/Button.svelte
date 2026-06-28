<script lang="ts">
import type { Snippet } from "svelte";
import type { HTMLButtonAttributes } from "svelte/elements";
import { cn } from "$lib/utils";

const VARIANTS = {
  ghost:
    "cursor-pointer rounded-lg border border-brand-cream/15 bg-brand-darker px-3 py-1.5 text-xs text-brand-cream/70 transition-colors hover:text-brand-cream",
  icon: "cursor-pointer rounded-full bg-brand-darker/85 p-2 shadow-[0_2px_12px_rgba(0,0,0,0.3)] backdrop-blur-sm transition-all hover:bg-brand-cream hover:text-brand-darker",
} as const;

let {
  variant = "ghost",
  class: className,
  children,
  ...restProps
}: HTMLButtonAttributes & {
  variant?: keyof typeof VARIANTS;
  children?: Snippet;
} = $props();
</script>

<button class={cn(VARIANTS[variant], className)} {...restProps}>
  {@render children?.()}
</button>
