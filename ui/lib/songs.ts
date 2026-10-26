// Song library bridge and song timing. Mirrors crates/song.
import { invoke } from "@tauri-apps/api/core";

export type Hand = "left" | "right";
export type SongNote = { key: number; start: number; length: number; velocity: number; hand: Hand | null };
export type Song = {
  version: number;
  title: string;
  artist: string;
  tempos: { beat: number; bpm: number }[];
  time_signatures: { beat: number; numerator: number; denominator: number }[];
  notes: SongNote[];
};
export type SongSummary = {
  id: string;
  title: string;
  artist: string;
  notes: number;
  seconds: number;
  hands: boolean;
  modified: number;
};
export type Imported = { songs: SongSummary[]; errors: string[] };

export const songs = {
  list: () => invoke<SongSummary[]>("list_songs"),
  get: (id: string) => invoke<Song>("get_song", { id }),
  importDialog: () => invoke<Imported>("import_songs_dialog"),
  importFiles: (paths: string[]) => invoke<Imported>("import_song_files", { paths }),
  rename: (id: string, title: string, artist: string) => invoke<SongSummary>("rename_song", { id, title, artist }),
  remove: (id: string) => invoke("delete_song", { id }),
  /** Saves over `id`, or as a new song when `id` is null. */
  save: (id: string | null, song: Song) => invoke<SongSummary>("save_song", { id, song }),
};

export function blankSong(): Song {
  return {
    version: 1,
    title: "Untitled",
    artist: "",
    tempos: [{ beat: 0, bpm: 90 }],
    time_signatures: [{ beat: 0, numerator: 4, denominator: 4 }],
    notes: [],
  };
}

/** Seconds from the start of the song to `beat`, following tempo changes (same as Song::seconds_at). */
export function secondsAt(song: Song, beat: number): number {
  let seconds = 0;
  for (let i = 0; i < song.tempos.length; i++) {
    const tempo = song.tempos[i];
    if (beat <= tempo.beat) break;
    const next = song.tempos[i + 1]?.beat ?? Infinity;
    seconds += ((Math.min(beat, next) - tempo.beat) * 60) / tempo.bpm;
  }
  return seconds;
}

/** Start time of every bar up to `endBeat`, in seconds. */
export function barTimes(song: Song, endBeat: number): number[] {
  const bars: number[] = [];
  const sigs = song.time_signatures;
  for (let i = 0; i < sigs.length; i++) {
    const beatsPerBar = (sigs[i].numerator * 4) / sigs[i].denominator;
    const until = Math.min(sigs[i + 1]?.beat ?? Infinity, endBeat);
    for (let beat = sigs[i].beat; beat < until; beat += beatsPerBar) bars.push(secondsAt(song, beat));
  }
  return bars;
}

export function formatTime(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}
