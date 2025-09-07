<script lang="ts">
    import type { Snippet } from "svelte";

    interface Props {
        children: Snippet;
    }

    let { children }: Props = $props();
</script>

<div
    class="absolute flex flex-row z-50 justify-evenly top-5 right-5 md:top-5 md:right-5 sm:top-2.5 sm:right-2.5"
>
    {@render children()}
</div>
