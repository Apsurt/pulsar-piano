<script lang="ts">
  // Your songs. Tap one to practice it; everything else about a song lives in its ⋯ menu.
  import Sheet from "../lib/Sheet.svelte";
  import TopBar from "../lib/TopBar.svelte";
  import { library, nav, openEditor, openSong, showError } from "../lib/app.svelte";
  import { fromHooktheory, type HooktheoryImport } from "../lib/hooktheory";
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

  // Import: a small menu (MIDI file, or Hooktheory's copied data pasted into a form).
  let importOpen = $state<"menu" | "hooktheory" | null>(null);
  let htText = $state("");
  let htTitle = $state("");
  let htArtist = $state("");
  let htBpm = $state(100);
  const htResult = $derived.by((): HooktheoryImport | string | null => {
    if (!htText.trim()) return null;
    try {
      return fromHooktheory(htText, htTitle, htArtist, Math.min(300, Math.max(20, htBpm || 100)));
    } catch (e) {
      return e instanceof Error ? e.message : String(e);
    }
  });

  function closeImport() {
    importOpen = null;
  }

  async function addFromHooktheory(e: SubmitEvent) {
    e.preventDefault();
    if (!htResult || typeof htResult === "string") return;
    importing = true;
    try {
      const saved = await songs.save(null, htResult.song);
      htText = htTitle = htArtist = "";
      closeImport();
      library.version++;
      openSong(saved.id);
    } catch (err) {
      showError(String(err));
    }
    importing = false;
  }

  async function importSongs() {
    closeImport();
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
      <div class="anchor">
        <button
          class="ghost"
          aria-expanded={importOpen !== null}
          disabled={importing}
          onclick={() => (importOpen = importOpen ? null : "menu")}
        >
          Import
        </button>
        <Sheet
          label="Import"
          open={importOpen !== null}
          onclose={closeImport}
          width={importOpen === "hooktheory" ? 400 : 230}
        >
          {#if importOpen === "hooktheory"}
            <form class="hooktheory" onsubmit={addFromHooktheory}>
              <div class="form-head">
                <span class="form-title">From Hooktheory</span>
                <span class="dim">On the song's Hooktheory page, click <b>Copy</b>, then paste here.</span>
              </div>
              <label class="sr-only" for="ht-text">Hooktheory data</label>
              <textarea id="ht-text" bind:value={htText} rows="4" placeholder="Paste here (Ctrl+V)"></textarea>
              {#if typeof htResult === "string"}
                <div class="error">{htResult}</div>
              {:else if htResult}
                <div class="found">
                  {htResult.melody} melody notes · {htResult.chords} chords · {htResult.key} · {htResult.bars} bars
                </div>
              {/if}
              <div class="fields">
                <input bind:value={htTitle} placeholder="Title" aria-label="Title" />
                <input bind:value={htArtist} placeholder="Artist" aria-label="Artist" />
              </div>
              <label class="bpm">
                <span class="dim">Tempo, as Hooktheory shows it</span>
                <input type="number" min="20" max="300" bind:value={htBpm} />
                <span class="dim">BPM</span>
              </label>
              <div class="form-actions">
                <button class="ghost small" type="button" onclick={() => (importOpen = "menu")}>Back</button>
                <button
                  class="primary small"
                  type="submit"
                  disabled={importing || !htResult || typeof htResult === "string" || !htTitle.trim()}
                >
                  Add song
                </button>
              </div>
            </form>
          {:else}
            <div class="menu">
              <button onclick={importSongs}>
                MIDI file…
                <span class="dim sub">.mid files from anywhere</span>
              </button>
              <button onclick={() => (importOpen = "hooktheory")}>
                From Hooktheory…
                <span class="dim sub">Paste a melody and its chords</span>
              </button>
            </div>
          {/if}
        </Sheet>
      </div>
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
  .menu .sub {
    display: block;
    margin-top: 2px;
    font-size: 12px;
  }
  .hooktheory {
    display: grid;
    gap: 10px;
  }
  .form-head {
    display: grid;
    gap: 4px;
    font-size: 12px;
    line-height: 1.4;
  }
  .form-head b {
    color: var(--snow);
    font-weight: 600;
  }
  .form-title {
    font-size: 15px;
    font-weight: 600;
  }
  textarea {
    font-family: var(--mono);
    font-size: 11px;
    color: var(--snow);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--line);
    border-radius: 10px;
    padding: 8px 10px;
    resize: none;
    user-select: text;
  }
  textarea:focus {
    outline: none;
    border-color: var(--pulse);
  }
  .found {
    font-size: 12px;
    color: var(--tint);
  }
  .error {
    font-size: 12px;
    line-height: 1.4;
    color: var(--coral);
  }
  .fields {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .bpm {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }
  .bpm input {
    width: 70px;
  }
  .form-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
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
