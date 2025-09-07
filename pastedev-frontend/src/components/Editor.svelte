<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { createVirtualizer } from "@tanstack/svelte-virtual";
    import { ApiClient, ApiError } from "../lib/api";
    import { navigate } from "../lib/router";
    import {
        SyntaxHighlighter,
        type HighlightResult,
    } from "../lib/highlighter";
    import Menu from "./Menu.svelte";
    import MenuButton from "./MenuButton.svelte";

    export let snippetId: string | null = null;

    // State
    let mode: "edit" | "view" | "loading" = snippetId ? "loading" : "edit";
    let content = "";
    let highlightedLines: Array<{ lineNumber: number; content: string }> = [];
    let textareaRef: HTMLTextAreaElement;
    let scrollContainer: HTMLDivElement;
    let error: string | null = null;

    const highlighter = new SyntaxHighlighter();

    // Create virtualizer for view mode
    let virtualizer = createVirtualizer({
        count: highlightedLines.length,
        getScrollElement: () => scrollContainer,
        estimateSize: () => 24,
        overscan: 10,
    });

    async function loadSnippet() {
        if (!snippetId) return;

        try {
            mode = "loading";
            error = null;

            content = await ApiClient.getSnippet(snippetId);

            if (!content.trim()) {
                highlightedLines = [
                    {
                        lineNumber: 1,
                        content:
                            '<span class="line text-gray-400">(empty file)</span>',
                    },
                ];
                mode = "view";
                return;
            }

            highlightedLines = await highlighter.highlight(content);
            mode = "view";

            document.title = `Snippet ${snippetId} - PasteDev`;
        } catch (err) {
            if (err instanceof ApiError) {
                error = err.message;
                if (err.status === 404) {
                    setTimeout(() => navigate("/"), 3000);
                }
            } else {
                error = "An unexpected error occurred";
            }
            console.error("Error loading snippet:", err);
            mode = "edit";
        }
    }

    async function saveSnippet() {
        if (!content.trim()) return;

        try {
            mode = "loading";
            error = null;

            const url = await ApiClient.createSnippet(content);
            console.log("Created snippet URL:", url);

            // Extract snippet ID from URL - handle both full URLs and relative paths
            let newSnippetId;
            if (url.includes("/")) {
                const urlParts = url.split("/");
                newSnippetId = urlParts[urlParts.length - 1];
            } else {
                newSnippetId = url; // In case API returns just the ID
            }

            // Update the current snippet ID and switch to view mode
            snippetId = newSnippetId;

            if (content.trim()) {
                highlightedLines = await highlighter.highlight(content);
            }

            mode = "view";
            navigate(`/${newSnippetId}`);
            document.title = `Snippet ${newSnippetId} - PasteDev`;
        } catch (err) {
            if (err instanceof ApiError) {
                error = err.message;
            } else {
                error = "An unexpected error occurred";
            }
            console.error("Error creating snippet:", err);
            mode = "edit";
        }
    }

    function editSnippet() {
        mode = "edit";
        document.title = "PasteDev";
        setTimeout(() => textareaRef?.focus(), 0);
    }

    function newSnippet() {
        content = "";
        navigate("/");
        mode = "edit";
        setTimeout(() => textareaRef?.focus(), 0);
    }

    function openRaw() {
        if (snippetId) {
            window.open(
                `http://localhost:8080/api/snippets/${snippetId}`,
                "_blank",
            );
        }
    }

    function handleKeyDown(event: KeyboardEvent) {
        if (event.ctrlKey && (event.key === "Enter" || event.key === "s")) {
            event.preventDefault();
            if (mode === "edit") {
                saveSnippet();
            }
        }
        if (event.key === "Escape") {
            if (mode === "view") {
                editSnippet();
            } else {
                newSnippet();
            }
        }
    }

    onMount(() => {
        document.addEventListener("keydown", handleKeyDown);
        if (snippetId) {
            loadSnippet();
        } else {
            setTimeout(() => textareaRef?.focus(), 0);
        }
    });

    onDestroy(() => {
        document.removeEventListener("keydown", handleKeyDown);
        highlighter.destroy();
    });

    // Reactive virtualizer update
    $: if (highlightedLines.length > 0 && scrollContainer && mode === "view") {
        virtualizer = createVirtualizer({
            count: highlightedLines.length,
            getScrollElement: () => scrollContainer,
            estimateSize: () => 24,
            overscan: 10,
        });
    }

    $: virtualItems = $virtualizer.getVirtualItems();
    $: totalSize = $virtualizer.getTotalSize();
</script>

<svelte:window on:keydown={handleKeyDown} />

<div class="relative w-full h-screen bg-[#282c34]">
    {#if error}
        <div
            class="fixed top-5 left-1/2 transform -translate-x-1/2 bg-red-600 text-white px-5 py-2.5 rounded-md z-50 font-mono animate-pulse"
        >
            {error}
        </div>
    {/if}

    <Menu>
        {#if mode === "loading"}
            <MenuButton>Loading...</MenuButton>
        {:else if mode === "edit"}
            <MenuButton onclick={saveSnippet}>Save</MenuButton>
        {:else if mode === "view"}
            <MenuButton onclick={openRaw}>Raw</MenuButton>
            <MenuButton onclick={editSnippet}>Edit</MenuButton>
            <MenuButton onclick={newSnippet}>New</MenuButton>
        {/if}
    </Menu>

    {#if mode === "edit"}
        <div class="relative w-full h-full">
            <div
                class="absolute z-10 top-5 left-4 w-8 text-[#abb2bf] font-mono text-base pointer-events-none"
            >
                {">"}
            </div>
            <textarea
                bind:this={textareaRef}
                bind:value={content}
                class="absolute px-8 py-5 w-full h-full border-none bg-transparent outline-none text-white font-mono text-sm resize-none leading-relaxed placeholder-gray-500"
                placeholder="Paste your code, text, or any content here..."
            ></textarea>
        </div>
    {:else if mode === "view"}
        <div class="w-full h-full overflow-hidden">
            <div
                class="w-full h-full overflow-auto font-mono text-sm leading-6"
                bind:this={scrollContainer}
            >
                <div class="relative w-full" style="height: {totalSize}px;">
                    {#each virtualItems as item}
                        <div
                            class="absolute top-0 left-0 w-full flex"
                            style="transform: translateY({item.start}px); height: {item.size}px;"
                        >
                            <div
                                class="bg-[#21252b] text-[#565c64] px-2.5 py-0 text-right border-r border-[#181a1f] select-none min-w-[60px] flex-shrink-0"
                            >
                                {highlightedLines[item.index]?.lineNumber ||
                                    item.index + 1}
                            </div>
                            <div
                                class="flex-1 px-4 py-0 bg-[#282c34] whitespace-pre overflow-hidden"
                            >
                                {@html highlightedLines[item.index]?.content ||
                                    ""}
                            </div>
                        </div>
                    {/each}
                </div>
            </div>
        </div>
    {:else if mode === "loading"}
        <div class="flex items-center justify-center w-full h-full">
            <div
                class="w-8 h-8 border-4 border-[#565c64] border-t-[#61dafb] rounded-full animate-spin"
            ></div>
        </div>
    {/if}
</div>

<style>
    /* Shiki theme overrides */
    :global(.shiki) {
        background: transparent !important;
    }

    :global(.shiki span) {
        color: var(--shiki-dark, inherit);
    }
</style>
