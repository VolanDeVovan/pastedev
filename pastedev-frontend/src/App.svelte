<script lang="ts">
    import { currentRoute, navigate } from "./lib/router";
    import Editor from "./components/Editor.svelte";

    $: route = $currentRoute;

    function goHome() {
        navigate("/");
    }
</script>

<main class="w-full h-screen overflow-hidden">
    {#if route.path === "/"}
        <Editor />
    {:else if route.path === "/snippet" && route.params.id}
        <Editor snippetId={route.params.id} />
    {:else}
        <!-- 404 page - redirect to home -->
        <div class="flex items-center justify-center w-full h-screen bg-[#282c34] text-white">
            <div class="text-center max-w-sm">
                <div class="text-7xl text-[#565c64] mb-4 font-mono">404</div>
                <h1 class="text-2xl font-bold mb-3 font-mono">Page Not Found</h1>
                <p class="text-[#abb2bf] mb-6 font-mono">The page you're looking for doesn't exist.</p>
                <button 
                    class="bg-[#282c34] text-white font-mono text-base px-4 py-2 border-none rounded-lg shadow-2xl cursor-pointer transition-colors duration-200 hover:text-white/60"
                    on:click={goHome}
                >
                    Go Home
                </button>
            </div>
        </div>
    {/if}
</main>
