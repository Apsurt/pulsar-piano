// App-wide state the screens share: sound, MIDI status, what's being played, navigation and messages.
import { engine, type InstrumentInfo, type MidiDevice, type Touch } from "./engine";
import { songs, type Song } from "./songs";

export const sound = $state({
  instruments: [] as InstrumentInfo[],
  current: null as string | null,
  loading: null as string | null,
  volume: 0,
  touch: "normal" as Touch,
});

export const status = $state({
  devices: [] as MidiDevice[],
  output: "",
  sampleRate: 0,
  latency: null as number | null,
  update: null as string | null,
  fatal: null as string | null,
});

/** What was just played, for the free-play readout. */
export const played = $state({
  lastNote: null as { key: number; velocity: number } | null,
  held: 0,
  /** Bumped on every note-on. */
  activity: 0,
  sustain: false,
});

/** Computer-keyboard playing: the octave, and when the keys were last used (to show a hint briefly). */
export const computerKeys = $state({ octave: 4, usedAt: 0 });

// ---- Practice preferences, kept between songs and launches ----------------------------------

const PRACTICE_PREFS = "pulsar.practice";

type PracticePrefs = {
  loop: boolean;
  leadIn: number;
  /** The tracks you play, per song id; songs not listed play every track. */
  tracks: Record<string, number[]>;
  /** Pulsar plays the tracks you don't. */
  accompany: boolean;
};

function loadPracticePrefs(): PracticePrefs {
  const defaults: PracticePrefs = { loop: false, leadIn: 2, tracks: {}, accompany: false };
  try {
    return { ...defaults, ...JSON.parse(localStorage.getItem(PRACTICE_PREFS) ?? "{}") };
  } catch {
    return defaults;
  }
}

export const practicePrefs = $state(loadPracticePrefs());

export function savePracticePrefs() {
  try {
    localStorage.setItem(PRACTICE_PREFS, JSON.stringify(practicePrefs));
  } catch {
    // Only a convenience; practice works with the defaults.
  }
}

/** Bumped whenever the song list may have changed. */
export const library = $state({ version: 0 });

export const toast = $state({ message: null as string | null });
let toastTimer: ReturnType<typeof setTimeout>;
export function showError(message: string) {
  toast.message = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (toast.message = null), 6000);
}

export async function refreshInstruments() {
  try {
    sound.instruments = await engine.instruments();
  } catch {
    // Keep the list we have; the engine reports real failures elsewhere.
  }
}

export function chooseInstrument(id: string) {
  if (id === sound.current || sound.loading) return;
  sound.loading = id;
  engine.load(id).catch((e) => {
    sound.loading = null;
    showError(String(e));
  });
}

export function setVolume(db: number) {
  sound.volume = db;
  engine.setVolume(db);
}

export function setTouch(touch: Touch) {
  sound.touch = touch;
  engine.setTouch(touch).catch((e) => showError(String(e)));
}

// ---- Navigation: a stack of screens; you go into things and back out -------------------------

export type Route =
  | { name: "home" }
  | { name: "play" }
  | { name: "songs" }
  | { name: "settings" }
  | { name: "practice"; id: string; song: Song }
  | { name: "edit"; id: string | null; song: Song | null };

class Navigation {
  stack = $state.raw<Route[]>([{ name: "home" }]);
  /** +1 going deeper, -1 coming back: the direction screens slide. */
  dir = $state(1);

  get route(): Route {
    return this.stack[this.stack.length - 1];
  }

  push(route: Route) {
    this.dir = 1;
    this.stack = [...this.stack, route];
  }

  back() {
    if (this.stack.length < 2) return;
    this.dir = -1;
    this.stack = this.stack.slice(0, -1);
  }

  set(stack: Route[], dir: 1 | -1) {
    this.dir = dir;
    this.stack = stack;
  }

  /** What the back button says: the name of the screen below. */
  get backLabel(): string {
    const below = this.stack[this.stack.length - 2];
    if (!below) return "";
    if (below.name === "practice") return below.song.title;
    return { home: "Home", play: "Play", songs: "Songs", settings: "Settings", edit: "Editor" }[below.name];
  }
}

export const nav = new Navigation();

const LAST_SONG = "pulsar.lastSong";

export function lastSongId(): string | null {
  try {
    return localStorage.getItem(LAST_SONG);
  } catch {
    return null;
  }
}

export async function openSong(id: string) {
  try {
    const song = await songs.get(id);
    try {
      localStorage.setItem(LAST_SONG, id);
    } catch {
      // Only used for "Continue" on the home screen.
    }
    nav.push({ name: "practice", id, song });
  } catch (e) {
    showError(String(e));
  }
}

export async function openEditor(id: string | null) {
  if (!id) return nav.push({ name: "edit", id: null, song: null });
  try {
    nav.push({ name: "edit", id, song: await songs.get(id) });
  } catch (e) {
    showError(String(e));
  }
}

/** Leaving the editor: back to where it was opened from (with the edits loaded), or on to practicing. */
export async function leaveEditor(id: string | null, practice: boolean) {
  const below = nav.stack.slice(0, -1);
  const previous = below.at(-1);
  if (!id) return nav.back();
  try {
    const song = await songs.get(id);
    if (previous?.name === "practice" && previous.id === id) {
      nav.set([...below.slice(0, -1), { name: "practice", id, song }], -1);
    } else if (practice) {
      nav.set([...below, { name: "practice", id, song }], 1);
    } else {
      nav.back();
    }
  } catch (e) {
    showError(String(e));
    nav.back();
  }
}
