<script lang="ts">
    interface Props {
        onclick?: () => void;
    }

    let { onclick }: Props = $props();
</script>

<button
    class="bg-[#282c34] text-white font-mono text-xl px-1 py-1 rounded-lg shadow-2xl ml-2.5 select-none cursor-pointer transition-colors duration-200 hover:text-white/60 focus-visible:outline-2 focus-visible:outline-blue-500 focus-visible:outline-offset-2 md:text-xl md:px-1 md:py-1 md:ml-2.5 sm:text-base sm:px-1 sm:py-1 sm:ml-2"
    on:click={onclick}
>
    <slot />
</button>
