// Typed bridge to the Rust side. Sound never goes through here: only control and visuals.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type InstrumentInfo = { id: string; name: string; kind: "soundfont" | "builtin" };
export type MidiDevice = { name: string; connected: boolean };
export type Status = {
  output_device: string;
  sample_rate: number;
  latency_ms: number;
  volume_db: number;
  devices: MidiDevice[];
  instrument: InstrumentInfo | null;
  sounds_dir: string;
};
export type InstrumentState = {
  state: "loading" | "ready" | "failed";
  instrument: InstrumentInfo;
  error: string | null;
};
/** `device`: played on a MIDI device rather than on screen / computer keys. */
export type Note = { key: number; velocity: number; device: boolean };

const on = <T>(event: string, cb: (payload: T) => void): Promise<UnlistenFn> =>
  listen<T>(event, (e) => cb(e.payload));

export const engine = {
  status: () => invoke<Status>("status"),
  instruments: () => invoke<InstrumentInfo[]>("instruments"),
  load: (id: string) => invoke<InstrumentInfo>("load_instrument", { id }),
  /** velocity 0 releases the key */
  note: (key: number, velocity: number) => invoke("note", { key, velocity }),
  pedal: (down: boolean) => invoke("pedal", { down }),
  setVolume: (db: number) => invoke("set_volume", { db }),
  openSoundsFolder: () => invoke("open_sounds_folder"),
  checkUpdate: () => invoke<string | null>("check_update"),
  installUpdate: () => invoke("install_update"),

  onNote: (cb: (note: Note) => void) => on("note", cb),
  onPedal: (cb: (down: boolean) => void) => on("pedal", cb),
  onDevices: (cb: (devices: MidiDevice[]) => void) => on("devices", cb),
  onInstrument: (cb: (state: InstrumentState) => void) => on("instrument", cb),
};
