<script lang="ts">
    import { currentRoute, navigate } from "./lib/router";
    import UploadForm from "./components/UploadForm.svelte";
    import SnippetView from "./components/SnippetView.svelte";

    $: route = $currentRoute;

    function goHome() {
        navigate("/");
    }
</script>

<main class="min-h-screen bg-gray-900">
    {#if route.path === "/"}
        <!-- Upload form page -->
        <UploadForm />
    {:else if route.path === "/snippet" && route.params.id}
        <!-- Snippet view -->
        <SnippetView snippetId={route.params.id} />
    {:else}
        <!-- 404 page - redirect to home -->
        <div
            class="min-h-screen bg-gray-900 text-gray-100 flex items-center justify-center"
        >
            <div class="text-center max-w-md">
                <div class="text-gray-400 text-6xl mb-4">404</div>
                <h1 class="text-2xl font-bold text-gray-200 mb-2">
                    Page Not Found
                </h1>
                <p class="text-gray-400 mb-4">
                    The page you're looking for doesn't exist.
                </p>
                <button
                    class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded transition-colors"
                    on:click={goHome}
                >
                    Go Home
                </button>
            </div>
        </div>
    {/if}
</main>
