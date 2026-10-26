<script lang="ts">
  // Your songs. Tap one to practice it; everything else about a song lives in its ⋯ menu.
  import Sheet from "../lib/Sheet.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { library, nav, openEditor, openSong, showError } from "../lib/app.svelte";
  import { formatTime, songs, type SongSummary } from "../lib/songs";

  let list = $state<SongSummary[]>([]);
  let loaded = $state(false);
  let query = $state("");
  let importing = $state(false);
  let menu = $state<string | null>(null);
  let confirmDelete = $state<string | null>(null);
  let renaming = $state<string | null>(null);
  let renameTitle = $state("");
  let renameArtist = $state("");

  const shown = $derived(
    list.filter((s) => `${s.title} ${s.artist}`.toLowerCase().includes(query.trim().toLowerCase())),
  );

  async function load() {
    try {
      list = await songs.list();
    } catch (e) {
      showError(String(e));
    }
    loaded = true;
  }

  $effect(() => {
    void library.version;
    load();
  });

  async function importSongs() {
    importing = true;
    try {
      const result = await songs.importDialog();
      if (result.errors.length) showError(result.errors.join("\n"));
      library.version++;
    } catch (e) {
      showError(String(e));
    }
    importing = false;
  }

  function closeMenu() {
    menu = null;
    confirmDelete = null;
  }

  function startRename(song: SongSummary) {
    closeMenu();
    renaming = song.id;
    renameTitle = song.title;
    renameArtist = song.artist;
  }

  async function saveRename(e: SubmitEvent) {
    e.preventDefault();
    if (!renaming || !renameTitle.trim()) return;
    try {
      await songs.rename(renaming, renameTitle, renameArtist);
      renaming = null;
      library.version++;
    } catch (err) {
      showError(String(err));
    }
  }

  async function remove(id: string) {
    if (confirmDelete !== id) {
      confirmDelete = id;
      return;
    }
    try {
      await songs.remove(id);
      closeMenu();
      library.version++;
    } catch (e) {
      showError(String(e));
    }
  }
</script>

<div class="library">
  <TopBar
    back={nav.backLabel}
    onback={() => nav.back()}
    title="Songs"
    subtitle={list.length ? `${list.length}` : undefined}
  >
    {#snippet actions()}
      {#if list.length > 6}
        <label class="search">
          <span class="sr-only">Search songs</span>
          <input type="search" placeholder="Search" bind:value={query} />
        </label>
      {/if}
      <button class="ghost" onclick={importSongs} disabled={importing}>Import</button>
      <button class="primary" onclick={() => openEditor(null)}>New song</button>
    {/snippet}
  </TopBar>

  <div class="content">
    {#if loaded && list.length === 0}
      <div class="empty">
        <h2>No songs yet</h2>
        <p class="dim">
          Make one from scratch, or import a MIDI file of a song you want to learn. Searching "<em>song name</em>
          piano midi" usually finds one, and you can drop it straight onto this window.
        </p>
        <div class="row">
          <button class="primary" onclick={() => openEditor(null)}>New song</button>
          <button class="ghost" onclick={importSongs} disabled={importing}>Import MIDI</button>
        </div>
      </div>
    {:else}
      <ul>
        {#each shown as song (song.id)}
          <li class:raised={menu === song.id}>
            {#if renaming === song.id}
              <form class="rename" onsubmit={saveRename}>
                <label>
                  <span class="sr-only">Title</span>
                  <input bind:value={renameTitle} placeholder="Title" />
                </label>
                <label>
                  <span class="sr-only">Artist</span>
                  <input bind:value={renameArtist} placeholder="Artist" />
                </label>
                <button class="primary small" type="submit">Save</button>
                <button class="ghost small" type="button" onclick={() => (renaming = null)}>Cancel</button>
              </form>
            {:else}
              <button class="open" onclick={() => openSong(song.id)}>
                <span class="title">{song.title}</span>
                <span class="dim">{song.artist || "Unknown artist"}</span>
                <span class="mono dim length">{formatTime(song.seconds)}</span>
              </button>
              <div class="anchor">
                <button
                  class="more"
                  aria-label="More for {song.title}"
                  aria-expanded={menu === song.id}
                  onclick={() => (menu = menu === song.id ? null : song.id)}
                >
                  <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden="true">
                    <circle cx="5" cy="12" r="1.6" /><circle cx="12" cy="12" r="1.6" /><circle cx="19" cy="12" r="1.6" />
                  </svg>
                </button>
                <Sheet label="{song.title} options" open={menu === song.id} onclose={closeMenu} width={200}>
                  <div class="menu">
                    <button onclick={() => openEditor(song.id)}>Edit notes</button>
                    <button onclick={() => startRename(song)}>Rename</button>
                    <button class="danger" onclick={() => remove(song.id)}>
                      {confirmDelete === song.id ? "Click again to delete" : "Delete"}
                    </button>
                  </div>
                </Sheet>
              </div>
            {/if}
          </li>
        {:else}
          {#if loaded}<li class="none dim">No songs match "{query}".</li>{/if}
        {/each}
      </ul>
    {/if}
  </div>
</div>

<style>
  .library {
    position: absolute;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr);
    background: rgba(10, 10, 15, 0.88);
    backdrop-filter: blur(18px);
  }
  .content {
    overflow-y: auto;
    padding: 4px max(18px, calc((100% - 760px) / 2)) 32px;
  }
  .dim {
    color: var(--mist);
  }
  button {
    font: inherit;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    color: var(--snow);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    padding: 8px 16px;
    border-radius: 999px;
    border: 0;
    background: var(--pulse);
    color: var(--void);
  }
  .ghost {
    padding: 8px 14px;
    border-radius: 999px;
    border: 1px solid var(--line);
    background: transparent;
    color: var(--mist);
  }
  .ghost:hover {
    color: var(--snow);
  }
  .small {
    padding: 6px 12px;
    font-size: 12px;
  }
  input {
    font: inherit;
    font-size: 13px;
    color: var(--snow);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 8px 12px;
    min-width: 0;
    user-select: text;
  }
  input:focus {
    outline: none;
    border-color: var(--pulse);
  }
  .search input {
    width: 160px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  li {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: 14px;
    transition: background 140ms ease;
  }
  li:hover,
  li.raised {
    background: rgba(255, 255, 255, 0.04);
  }
  li.raised {
    z-index: 2;
  }
  li.none {
    padding: 16px;
    font-size: 13px;
  }
  .open {
    flex: 1;
    min-width: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 16px;
    row-gap: 2px;
    padding: 12px 14px;
    border: 0;
    border-radius: 14px;
    background: transparent;
    text-align: left;
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .open .dim {
    font-size: 13px;
    font-weight: 400;
  }
  .length {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    font-size: 12px;
  }
  .anchor {
    position: relative;
    margin-right: 6px;
  }
  .more {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border: 0;
    border-radius: 50%;
    background: transparent;
    color: var(--mist);
    opacity: 0;
    transition: opacity 120ms ease;
  }
  .more svg {
    fill: currentColor;
  }
  li:hover .more,
  .more:focus-visible,
  .more[aria-expanded="true"] {
    opacity: 1;
  }
  .more:hover {
    color: var(--snow);
    background: rgba(255, 255, 255, 0.06);
  }
  .menu {
    display: grid;
    gap: 2px;
    margin: -6px;
  }
  .menu button {
    padding: 9px 12px;
    border: 0;
    border-radius: 10px;
    background: transparent;
    text-align: left;
    font-weight: 400;
    font-size: 14px;
  }
  .menu button:hover {
    background: rgba(255, 255, 255, 0.07);
  }
  .menu .danger {
    color: var(--coral);
  }
  .rename {
    flex: 1;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 8px;
    flex-wrap: wrap;
  }
  .rename label {
    flex: 1;
    min-width: 140px;
    display: flex;
  }
  .rename input {
    flex: 1;
  }

  .empty {
    margin-top: 10vh;
    display: grid;
    justify-items: center;
    gap: 12px;
    text-align: center;
  }
  .empty h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .empty p {
    margin: 0;
    max-width: 440px;
    font-size: 14px;
    line-height: 1.5;
  }
  .row {
    display: flex;
    gap: 8px;
  }
</style>
