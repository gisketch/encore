<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { onMount } from "svelte";
  import LibraryGroupCard from "./LibraryGroup.svelte";
  import { formatRollup } from "./libraryFormat";
  import { filterIndex, isGroupVisible } from "./librarySearch";
  import type { LibraryIndex } from "./libraryTypes";

  const TINT_COUNT = 4;
  const RECENT_EXPANDED_COUNT = 2;

  let index = $state<LibraryIndex>({ groups: [], totalCount: 0, totalBytes: 0 });
  let expandedGroups = $state<number[]>([]);
  let errorCode = $state<string | null>(null);
  let query = $state("");
  let displayedIndex = $derived(filterIndex(index, query));

  onMount(() => {
    if (!isTauri()) return;
    void refresh();
    // The window hides instead of closing (global close-as-hide handler),
    // so a stale list would otherwise survive across visits; rescan
    // whenever the window regains focus, same as the Settings sections.
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onFocusChanged(({ payload: focused }) => {
        if (focused) void refresh();
      })
      .then((stop) => {
        unlisten = stop;
      });
    return () => unlisten?.();
  });

  async function refresh() {
    try {
      index = await invoke<LibraryIndex>("library_index");
    } catch (error) {
      errorCode = typeof error === "string" ? error : "library_index_failed";
    }
  }

  function isExpanded(groupIndex: number) {
    return isGroupVisible(query, groupIndex, expandedGroups, RECENT_EXPANDED_COUNT);
  }

  function expand(groupIndex: number) {
    expandedGroups = [...expandedGroups, groupIndex];
  }

  async function openReplay(id: string) {
    errorCode = null;
    try {
      await invoke("open_editor_window", { id });
    } catch (error) {
      errorCode = typeof error === "string" ? error : "editor_window_open_failed";
    }
  }

  function handleDeleted() {
    void refresh();
  }

  function openFolder() {
    errorCode = null;
    void invoke("open_export_folder").catch((error) => {
      errorCode = typeof error === "string" ? error : "export_folder_open_failed";
    });
  }

  function closeWindow() {
    void getCurrentWindow().close();
  }
</script>

<svelte:head><title>Library</title></svelte:head>

<main class="library-shell">
  <section class="library-card-surface">
    <header class="library-titlebar" data-tauri-drag-region="deep">
      <span class="traffic-lights">
        <button
          type="button"
          class="traffic-dot traffic-dot--close"
          title="Close"
          aria-label="Close library"
          onclick={closeWindow}
        ></button>
        <i class="traffic-dot traffic-dot--minimize" aria-hidden="true"></i>
        <i class="traffic-dot traffic-dot--zoom" aria-hidden="true"></i>
      </span>
      <span class="buffer-mark" aria-hidden="true"><i></i><i></i><i></i></span>
      <h1 class="library-title">Library</h1>
      <span class="library-rollup">
        {formatRollup(displayedIndex.totalCount, displayedIndex.totalBytes)}
      </span>
      <span class="spacer" aria-hidden="true"></span>
      <label class="library-search">
        <span class="library-search__icon" aria-hidden="true"></span>
        <input
          type="search"
          class="library-search__input"
          placeholder="Search replays"
          bind:value={query}
        />
      </label>
      <button type="button" class="library-pill" onclick={openFolder}>Open Folder ↗</button>
    </header>

    <div class="library-body">
      {#if errorCode}
        <p class="library-error" role="alert">{errorCode}</p>
      {/if}
      {#if displayedIndex.groups.length === 0}
        <p class="library-empty">
          {query.trim().length > 0
            ? "No replays match your search."
            : "No replays yet — Save Replay from the bar to see one here."}
        </p>
      {:else}
        {#each displayedIndex.groups as group, groupIndex (group.label)}
          <LibraryGroupCard
            {group}
            tint={groupIndex % TINT_COUNT}
            expanded={isExpanded(groupIndex)}
            onExpand={() => expand(groupIndex)}
            onOpen={openReplay}
            onDeleted={handleDeleted}
          />
        {/each}
      {/if}
    </div>
  </section>
</main>
