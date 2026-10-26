<script lang="ts">
  // The song library: import, browse, rename, delete, open.
  import { formatTime, songs, type SongSummary } from "../lib/songs";

  let {
    onopen,
    onerror,
    refresh = 0,
  }: {
    onopen: (id: string) => void;
    onerror: (message: string) => void;
    /** Bumped by the app after songs are dropped onto the window. */
    refresh?: number;
  } = $props();

  let list = $state<SongSummary[]>([]);
  let loaded = $state(false);
  let query = $state("");
  let importing = $state(false);
  let editing = $state<string | null>(null);
  let editTitle = $state("");
  let editArtist = $state("");
  let confirmDelete = $state<string | null>(null);

  const shown = $derived(
    list.filter((s) => `${s.title} ${s.artist}`.toLowerCase().includes(query.trim().toLowerCase())),
  );

  async function load() {
    try {
      list = await songs.list();
    } catch (e) {
      onerror(String(e));
    }
    loaded = true;
  }

  $effect(() => {
    void refresh;
    load();
  });

  async function importSongs() {
    importing = true;
    try {
      const result = await songs.importDialog();
      if (result.errors.length) onerror(result.errors.join("\n"));
      await load();
    } catch (e) {
      onerror(String(e));
    }
    importing = false;
  }

  function startEdit(song: SongSummary) {
    editing = song.id;
    editTitle = song.title;
    editArtist = song.artist;
    confirmDelete = null;
  }

  async function saveEdit(e: SubmitEvent) {
    e.preventDefault();
    if (!editing || !editTitle.trim()) return;
    try {
      await songs.rename(editing, editTitle, editArtist);
      editing = null;
      await load();
    } catch (err) {
      onerror(String(err));
    }
  }

  /** Two clicks: the first arms the button, the second deletes. */
  async function remove(id: string) {
    if (confirmDelete !== id) {
      confirmDelete = id;
      return;
    }
    try {
      await songs.remove(id);
      confirmDelete = null;
      await load();
    } catch (e) {
      onerror(String(e));
    }
  }
</script>

<section class="library">
  <header>
    <div>
      <h1>Songs</h1>
      <p class="dim">
        {list.length}
        {list.length === 1 ? "song" : "songs"} · drop .mid files anywhere to add them
      </p>
    </div>
    <div class="actions">
      {#if list.length > 0}
        <label>
          <span class="sr-only">Search songs</span>
          <input type="search" placeholder="Search" bind:value={query} />
        </label>
      {/if}
      <button class="primary" onclick={importSongs} disabled={importing}>Import MIDI</button>
    </div>
  </header>

  {#if loaded && list.length === 0}
    <div class="empty">
      <h2>No songs yet</h2>
      <p class="dim">
        Find a MIDI file of a song you want to learn (searching "<em>song name</em> piano midi" usually works), then
        import it or drop it onto this window.
      </p>
      <button class="primary" onclick={importSongs} disabled={importing}>Import MIDI</button>
    </div>
  {:else}
    <ul>
      {#each shown as song (song.id)}
        <li>
          {#if editing === song.id}
            <form class="edit" onsubmit={saveEdit}>
              <label>
                <span class="sr-only">Title</span>
                <input bind:value={editTitle} placeholder="Title" />
              </label>
              <label>
                <span class="sr-only">Artist</span>
                <input bind:value={editArtist} placeholder="Artist" />
              </label>
              <button class="primary small" type="submit">Save</button>
              <button class="ghost small" type="button" onclick={() => (editing = null)}>Cancel</button>
            </form>
          {:else}
            <button class="open" onclick={() => onopen(song.id)}>
              <span class="title">{song.title}</span>
              <span class="dim">{song.artist || "Unknown artist"}</span>
              <span class="mono dim meta">
                {formatTime(song.seconds)} · {song.notes} notes{song.hands ? " · L+R" : ""}
              </span>
            </button>
            <div class="row-actions">
              <button class="ghost small" onclick={() => startEdit(song)}>Rename</button>
              <button
                class="ghost small"
                class:danger={confirmDelete === song.id}
                onclick={() => remove(song.id)}
                onblur={() => {
                  if (confirmDelete === song.id) confirmDelete = null;
                }}
              >
                {confirmDelete === song.id ? "Delete?" : "Delete"}
              </button>
            </div>
          {/if}
        </li>
      {:else}
        {#if loaded}<li class="dim none">No songs match "{query}".</li>{/if}
      {/each}
    </ul>
  {/if}
</section>

<style>
  .library {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    padding: 32px max(24px, calc((100% - 880px) / 2)) 40px;
    background: rgba(10, 10, 15, 0.86);
    backdrop-filter: blur(18px);
  }

  header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    gap: 24px;
    flex-wrap: wrap;
    margin-bottom: 24px;
  }
  h1 {
    margin: 0 0 4px;
    font-size: 32px;
    font-weight: 600;
    letter-spacing: -0.03em;
  }
  p {
    margin: 0;
    font-size: 13px;
  }
  .dim {
    color: var(--mist);
  }
  .actions {
    display: flex;
    gap: 10px;
    align-items: center;
  }

  input {
    font: inherit;
    font-size: 13px;
    color: var(--snow);
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid var(--line-strong);
    border-radius: 10px;
    padding: 9px 12px;
    min-width: 0;
    user-select: text;
  }
  input:focus {
    outline: none;
    border-color: var(--pulse);
  }

  button {
    font-size: 13px;
    font-weight: 500;
    border-radius: 999px;
    padding: 9px 18px;
    cursor: pointer;
    border: 1px solid var(--line-strong);
    background: rgba(255, 255, 255, 0.06);
    color: var(--snow);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .primary {
    background: var(--pulse);
    border-color: transparent;
    color: var(--void);
    box-shadow: 0 0 22px rgba(127, 212, 255, 0.3);
  }
  .ghost {
    background: transparent;
    border-color: transparent;
    color: var(--mist);
  }
  .ghost:hover {
    color: var(--snow);
    background: rgba(255, 255, 255, 0.06);
  }
  .small {
    padding: 6px 12px;
    font-size: 12px;
  }
  .danger {
    color: var(--coral);
    border-color: rgba(255, 122, 89, 0.5);
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  li {
    display: flex;
    align-items: center;
    gap: 8px;
    border-radius: 16px;
    background: rgba(22, 22, 29, 0.8);
    border: 1px solid var(--line);
    padding: 6px 8px 6px 6px;
    transition: border-color 160ms ease;
  }
  li:hover {
    border-color: var(--line-strong);
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
    grid-template-rows: auto auto;
    column-gap: 16px;
    row-gap: 2px;
    text-align: left;
    padding: 10px 12px;
    border: 0;
    border-radius: 12px;
    background: transparent;
  }
  .open:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .title {
    font-size: 15px;
    font-weight: 600;
    letter-spacing: -0.01em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .open .dim {
    font-size: 13px;
  }
  .meta {
    grid-column: 2;
    grid-row: 1 / span 2;
    align-self: center;
    font-size: 12px;
  }
  .row-actions {
    display: flex;
    gap: 2px;
  }
  .edit {
    flex: 1;
    display: flex;
    gap: 8px;
    align-items: center;
    padding: 6px;
    flex-wrap: wrap;
  }
  .edit label {
    flex: 1;
    min-width: 140px;
    display: flex;
  }
  .edit input {
    flex: 1;
  }

  .empty {
    margin-top: 12vh;
    text-align: center;
    display: grid;
    justify-items: center;
    gap: 12px;
  }
  .empty h2 {
    margin: 0;
    font-size: 24px;
    font-weight: 600;
    letter-spacing: -0.02em;
  }
  .empty p {
    max-width: 420px;
    line-height: 1.5;
  }
</style>
