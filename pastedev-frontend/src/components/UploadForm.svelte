<script lang="ts">
    import { ApiClient, ApiError } from "../lib/api";
    import { navigate } from "../lib/router";

    let content = "";
    let isSubmitting = false;
    let error: string | null = null;
    let successUrl: string | null = null;

    async function handleSubmit() {
        if (!content.trim()) {
            error = "Content cannot be empty";
            return;
        }

        try {
            isSubmitting = true;
            error = null;
            successUrl = null;

            const url = await ApiClient.createSnippet(content);
            successUrl = url;
            
            // Extract snippet ID from URL for navigation
            const urlParts = url.split('/');
            const snippetId = urlParts[urlParts.length - 1];
            
            // Navigate to the new snippet after a short delay
            setTimeout(() => {
                navigate(`/${snippetId}`);
            }, 2000);
        } catch (err) {
            if (err instanceof ApiError) {
                error = err.message;
            } else {
                error = "An unexpected error occurred";
            }
            console.error("Error creating snippet:", err);
        } finally {
            isSubmitting = false;
        }
    }

    function handleKeyDown(event: KeyboardEvent) {
        if (event.ctrlKey && event.key === 'Enter') {
            handleSubmit();
        }
    }

    function copyToClipboard() {
        if (successUrl) {
            navigator.clipboard.writeText(successUrl);
        }
    }
</script>

<div class="min-h-screen bg-gray-900 text-gray-100">
    <div class="container mx-auto px-4 py-8 max-w-4xl">
        <div class="text-center mb-8">
            <h1 class="text-4xl font-bold text-blue-400 mb-2">PasteDev</h1>
            <p class="text-lg text-gray-300">Share code snippets instantly</p>
        </div>

        {#if successUrl}
            <!-- Success State -->
            <div class="bg-green-900 border border-green-700 rounded-lg p-6 mb-6">
                <div class="flex items-center justify-between">
                    <div>
                        <h2 class="text-lg font-semibold text-green-100 mb-2">
                            Snippet Created Successfully!
                        </h2>
                        <p class="text-green-200 mb-3">
                            Your snippet is available at:
                        </p>
                        <div class="bg-green-800 rounded p-3 font-mono text-sm break-all">
                            {successUrl}
                        </div>
                    </div>
                    <button
                        class="bg-green-600 hover:bg-green-700 text-white px-4 py-2 rounded transition-colors ml-4"
                        on:click={copyToClipboard}
                    >
                        Copy URL
                    </button>
                </div>
                <p class="text-green-300 text-sm mt-3">
                    Redirecting to snippet in 2 seconds...
                </p>
            </div>
        {/if}

        {#if error}
            <!-- Error State -->
            <div class="bg-red-900 border border-red-700 rounded-lg p-4 mb-6">
                <div class="flex items-center">
                    <div class="text-red-400 mr-3">⚠️</div>
                    <div>
                        <h3 class="text-red-100 font-semibold">Error</h3>
                        <p class="text-red-200">{error}</p>
                    </div>
                </div>
            </div>
        {/if}

        <!-- Upload Form -->
        <div class="bg-gray-800 border border-gray-700 rounded-lg p-6">
            <form on:submit|preventDefault={handleSubmit}>
                <div class="mb-4">
                    <label
                        for="content"
                        class="block text-sm font-medium text-gray-200 mb-2"
                    >
                        Paste your content
                    </label>
                    <textarea
                        id="content"
                        bind:value={content}
                        on:keydown={handleKeyDown}
                        placeholder="Paste your code, text, or any content here..."
                        class="w-full h-64 bg-gray-900 border border-gray-600 rounded-lg px-4 py-3 text-gray-100 font-mono text-sm placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-blue-500 resize-y"
                        disabled={isSubmitting}
                    ></textarea>
                </div>

                <div class="flex items-center justify-between">
                    <div class="text-sm text-gray-400">
                        Press Ctrl+Enter to submit quickly
                    </div>
                    <button
                        type="submit"
                        disabled={isSubmitting || !content.trim()}
                        class="bg-blue-600 hover:bg-blue-700 disabled:bg-gray-600 disabled:cursor-not-allowed text-white px-6 py-2 rounded-lg transition-colors flex items-center"
                    >
                        {#if isSubmitting}
                            <div
                                class="animate-spin rounded-full h-4 w-4 border-b-2 border-white mr-2"
                            ></div>
                            Creating...
                        {:else}
                            Create Snippet
                        {/if}
                    </button>
                </div>
            </form>
        </div>

        <!-- Instructions -->
        <div class="mt-8 text-center text-gray-400 text-sm">
            <p>
                Your snippets are ephemeral and will be automatically deleted
                after viewing.
            </p>
            <p class="mt-1">
                Built with Svelte, TailwindCSS, Shiki, and TanStack Virtual
            </p>
        </div>
    </div>
</div>