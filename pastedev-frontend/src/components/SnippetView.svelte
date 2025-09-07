<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { createVirtualizer } from "@tanstack/svelte-virtual";
    import { ApiClient, ApiError } from "../lib/api";
    import { navigate } from "../lib/router";
    import {
        SyntaxHighlighter,
        type HighlightResult,
    } from "../lib/highlighter";

    export let snippetId: string;

    let scrollContainer: HTMLDivElement;
    let loading = true;
    let error: string | null = null;
    let content = "";
    let highlightedLines: Array<{ lineNumber: number; content: string }> = [];

    const highlighter = new SyntaxHighlighter();

    // Create virtualizer - will be updated reactively
    let virtualizer = createVirtualizer({
        count: highlightedLines.length,
        getScrollElement: () => scrollContainer,
        estimateSize: () => 24, // Estimated line height
        overscan: 10,
    });

    async function loadSnippet() {
        try {
            loading = true;
            error = null;

            // Fetch the snippet content
            console.log("Loading snippet:", snippetId);
            content = await ApiClient.getSnippet(snippetId);

            if (!content.trim()) {
                highlightedLines = [
                    {
                        lineNumber: 1,
                        content:
                            '<span class="line text-gray-400">(empty file)</span>',
                    },
                ];
                loading = false;
                return;
            }

            // Start syntax highlighting in the background
            const result = await highlighter.highlight(content);
            highlightedLines = result.lines;

            // Update document title
            document.title = `Snippet ${snippetId} - PasteDev`;
        } catch (err) {
            if (err instanceof ApiError) {
                error = err.message;
                if (err.status === 404) {
                    document.title = "Snippet Not Found - PasteDev";
                    // Redirect to home after showing error briefly
                    setTimeout(() => {
                        navigate("/");
                    }, 3000);
                }
            } else {
                error = "An unexpected error occurred";
            }
            console.error("Error loading snippet:", err);
        } finally {
            loading = false;
        }
    }

    onMount(() => {
        loadSnippet();
    });

    onDestroy(() => {
        highlighter.destroy();
    });

    // Reactive statement to update virtualizer when lines change
    $: if (highlightedLines.length > 0 && scrollContainer) {
        virtualizer = createVirtualizer({
            count: highlightedLines.length,
            getScrollElement: () => scrollContainer,
            estimateSize: () => 24,
            overscan: 10,
        });
    }

    $: virtualItems = $virtualizer.getVirtualItems();
    $: totalSize = $virtualizer.getTotalSize();

    function goHome() {
        navigate("/");
    }
</script>

<div class="min-h-screen bg-gray-900 text-gray-100">
    {#if loading}
        <div class="flex items-center justify-center min-h-screen">
            <div class="text-center">
                <div
                    class="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-400 mx-auto mb-4"
                ></div>
                <p class="text-gray-400">Loading snippet...</p>
            </div>
        </div>
    {:else if error}
        <div class="flex items-center justify-center min-h-screen">
            <div class="text-center max-w-md">
                <div class="text-red-400 text-6xl mb-4">⚠️</div>
                <h1 class="text-2xl font-bold text-red-400 mb-2">Error</h1>
                <p class="text-gray-300 mb-4">{error}</p>
                {#if error.includes("not found")}
                    <p class="text-gray-500 text-sm mb-4">
                        Redirecting to home in 3 seconds...
                    </p>
                {/if}
                <button
                    class="bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded transition-colors"
                    on:click={goHome}
                >
                    Go Home
                </button>
            </div>
        </div>
    {:else}
        <!-- Snippet content with virtual scrolling -->
        <div class="flex flex-col h-screen">
            <!-- Header -->
            <header class="bg-gray-800 border-b border-gray-700 px-4 py-3">
                <div class="flex items-center justify-between">
                    <h1 class="text-lg font-semibold">
                        <span class="text-gray-400">Snippet:</span>
                        <span class="text-blue-400 font-mono">{snippetId}</span>
                    </h1>
                    <div class="flex items-center gap-4">
                        <div class="text-sm text-gray-400">
                            {highlightedLines.length} lines
                        </div>
                        <button
                            class="bg-blue-600 hover:bg-blue-700 text-white px-3 py-1 rounded text-sm transition-colors"
                            on:click={goHome}
                        >
                            New Snippet
                        </button>
                    </div>
                </div>
            </header>

            <!-- Virtual scrolled content -->
            <div
                class="flex-1 overflow-auto font-mono text-sm leading-6"
                bind:this={scrollContainer}
            >
                <div class="relative" style="height: {totalSize}px;">
                    {#each virtualItems as item}
                        <div
                            class="absolute top-0 left-0 w-full flex"
                            style="transform: translateY({item.start}px); height: {item.size}px;"
                        >
                            <!-- Line number -->
                            <div
                                class="bg-gray-800 text-gray-500 px-3 py-1 text-right border-r border-gray-700 select-none"
                                style="min-width: {Math.max(
                                    3,
                                    highlightedLines.length.toString().length +
                                        1,
                                )}ch;"
                            >
                                {highlightedLines[item.index]?.lineNumber ||
                                    item.index + 1}
                            </div>
                            <!-- Line content -->
                            <div class="flex-1 px-4 py-1 bg-gray-900">
                                {@html highlightedLines[item.index]?.content ||
                                    ""}
                            </div>
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {/if}
</div>

<style>
    /* Shiki theme variables for dark theme */
    :global(.shiki) {
        background: transparent !important;
    }

    :global(.shiki span) {
        color: var(--shiki-dark, inherit);
    }

    :global(.line) {
        display: inline;
    }
</style>