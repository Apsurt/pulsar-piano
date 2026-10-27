mod settings;
mod songs;

use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};

use pulsar_engine::{
    CC_SUSTAIN, Engine, EngineEvent, InstrumentInfo, MidiEvent, Source, Status, Touch,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;
use tauri_plugin_updater::UpdaterExt;

use settings::Settings;

/// The audio engine runs on its own threads; the shell only forwards commands and events.
struct Pulsar {
    engine: Result<Arc<Engine>, String>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
}

impl Pulsar {
    fn engine(&self) -> Result<&Arc<Engine>, String> {
        self.engine
            .as_ref()
            .map_err(|e| format!("audio engine unavailable: {e}"))
    }

    fn update_settings(&self, edit: impl FnOnce(&mut Settings)) {
        if let Ok(mut settings) = self.settings.lock() {
            edit(&mut settings);
            settings.save(&self.settings_path);
        }
    }
}

#[derive(Clone, Serialize)]
struct Note {
    key: u8,
    /// 0 means released.
    velocity: u8,
    /// Played on a MIDI device rather than on screen / computer keys.
    device: bool,
}

/// A CC for the UI to act on: a transport pad on the control port (127 pressed, 0 released),
/// or a fader/knob on the keyboard port.
#[derive(Clone, Serialize)]
struct Control {
    controller: u8,
    value: u8,
    /// "control" or "keyboard".
    port: &'static str,
}

#[derive(Clone, Serialize)]
struct InstrumentState {
    state: &'static str,
    instrument: InstrumentInfo,
    error: Option<String>,
}

#[tauri::command]
fn status(pulsar: State<Pulsar>) -> Result<Status, String> {
    Ok(pulsar.engine()?.status())
}

#[tauri::command]
fn instruments(pulsar: State<Pulsar>) -> Result<Vec<InstrumentInfo>, String> {
    Ok(pulsar.engine()?.instruments())
}

#[tauri::command]
async fn load_instrument(app: AppHandle, id: String) -> Result<InstrumentInfo, String> {
    let pulsar = app.state::<Pulsar>();
    let engine = pulsar.engine()?.clone();
    let info = tauri::async_runtime::spawn_blocking(move || engine.load_instrument(&id))
        .await
        .map_err(|e| e.to_string())??;
    pulsar.update_settings(|s| s.instrument = Some(info.id.clone()));
    Ok(info)
}

/// On-screen and computer-keyboard input. Hardware MIDI never comes through here.
#[tauri::command]
fn note(pulsar: State<Pulsar>, key: u8, velocity: u8) -> Result<(), String> {
    let key = key.min(127);
    pulsar.engine()?.send(match velocity.min(127) {
        0 => MidiEvent::NoteOff {
            channel: 0,
            key,
            velocity: 0,
        },
        velocity => MidiEvent::NoteOn {
            channel: 0,
            key,
            velocity,
        },
    });
    Ok(())
}

#[tauri::command]
fn pedal(pulsar: State<Pulsar>, down: bool) -> Result<(), String> {
    let value = if down { 127 } else { 0 };
    pulsar.engine()?.send(MidiEvent::ControlChange {
        channel: 0,
        controller: CC_SUSTAIN,
        value,
    });
    Ok(())
}

#[tauri::command]
fn set_volume(pulsar: State<Pulsar>, db: f32) -> Result<(), String> {
    pulsar.engine()?.set_volume_db(db);
    pulsar.update_settings(|s| s.volume_db = db);
    Ok(())
}

/// How hard the keys need to be played: light, normal, heavy or fixed.
#[tauri::command]
fn set_touch(pulsar: State<Pulsar>, touch: Touch) -> Result<(), String> {
    pulsar.engine()?.set_touch(touch);
    pulsar.update_settings(|s| s.touch = touch);
    Ok(())
}

/// While practicing, the touch strips move through the song instead of bending the sound.
#[tauri::command]
fn set_strips_as_controls(pulsar: State<Pulsar>, on: bool) -> Result<(), String> {
    pulsar.engine()?.set_strips_as_controls(on);
    Ok(())
}

/// While practicing with wrong keys silent: only these keyboard keys sound (`None`: all of them).
#[tauri::command]
fn set_key_filter(pulsar: State<Pulsar>, keys: Option<Vec<u8>>) -> Result<(), String> {
    pulsar.engine()?.set_key_filter(keys.as_deref());
    Ok(())
}

#[tauri::command]
fn open_sounds_folder(app: AppHandle, pulsar: State<Pulsar>) -> Result<(), String> {
    let dir = pulsar.engine()?.status().sounds_dir;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(|e| e.to_string())
}

/// Returns the newer version on GitHub Releases, if any. Dev builds never update.
#[tauri::command]
async fn check_update(app: AppHandle) -> Result<Option<String>, String> {
    if cfg!(debug_assertions) {
        return Ok(None);
    }
    let update = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    Ok(update.map(|u| u.version))
}

#[tauri::command]
async fn install_update(app: AppHandle) -> Result<(), String> {
    if cfg!(debug_assertions) {
        return Ok(());
    }
    let Some(update) = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?
    else {
        return Ok(());
    };
    update
        .download_and_install(|_, _| {}, || {})
        .await
        .map_err(|e| e.to_string())?;
    app.restart()
}

/// Turns engine events into webview events. Only what the UI draws is forwarded.
fn forward_events(app: AppHandle, events: Receiver<EngineEvent>) {
    std::thread::spawn(move || {
        for event in events {
            let _ = match event {
                EngineEvent::Midi(
                    MidiEvent::ControlChange {
                        controller, value, ..
                    },
                    source @ (Source::Control | Source::Device),
                ) if controller != CC_SUSTAIN => {
                    let port = if source == Source::Control {
                        "control"
                    } else {
                        "keyboard"
                    };
                    app.emit(
                        "control",
                        Control {
                            controller,
                            value,
                            port,
                        },
                    )
                }
                EngineEvent::Midi(_, Source::Control) => Ok(()),
                EngineEvent::Midi(MidiEvent::NoteOn { key, velocity, .. }, source) => {
                    let device = source == Source::Device;
                    app.emit(
                        "note",
                        Note {
                            key,
                            velocity,
                            device,
                        },
                    )
                }
                EngineEvent::Midi(MidiEvent::NoteOff { key, .. }, source) => {
                    let device = source == Source::Device;
                    app.emit(
                        "note",
                        Note {
                            key,
                            velocity: 0,
                            device,
                        },
                    )
                }
                EngineEvent::Midi(
                    MidiEvent::ControlChange {
                        controller: CC_SUSTAIN,
                        value,
                        ..
                    },
                    _,
                ) => app.emit("pedal", value >= 64),
                // The left touch strip: -1 (down) to 1 (up), springing back to 0.
                EngineEvent::Midi(MidiEvent::PitchBend { value, .. }, Source::Device) => {
                    app.emit("bend", (f32::from(value) - 8192.0) / 8192.0)
                }
                EngineEvent::Midi(..) => Ok(()),
                EngineEvent::Devices(devices) => app.emit("devices", devices),
                EngineEvent::InstrumentLoading(instrument) => app.emit(
                    "instrument",
                    InstrumentState {
                        state: "loading",
                        instrument,
                        error: None,
                    },
                ),
                EngineEvent::InstrumentChanged(instrument) => app.emit(
                    "instrument",
                    InstrumentState {
                        state: "ready",
                        instrument,
                        error: None,
                    },
                ),
                EngineEvent::InstrumentFailed { instrument, error } => app.emit(
                    "instrument",
                    InstrumentState {
                        state: "failed",
                        instrument,
                        error: Some(error),
                    },
                ),
            };
        }
    });
}

/// Loads the last-used instrument, else the first SoundFont, else the built-in synth.
fn restore_instrument(engine: Arc<Engine>, saved: Option<String>) {
    std::thread::spawn(move || {
        let available = engine.instruments();
        let id = saved
            .filter(|id| available.iter().any(|i| &i.id == id))
            .or_else(|| available.first().map(|i| i.id.clone()));
        if let Some(id) = id
            && let Err(e) = engine.load_instrument(&id)
        {
            eprintln!("could not load {id}: {e}");
        }
    });
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Must be registered first. Dev and release have different identifiers, so they don't block each other.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data = app.path().app_data_dir()?;
            app.manage(songs::Songs(pulsar_song::Library::open(
                data.join("songs"),
            )?));
            let sounds = data.join("sounds");
            eprintln!("sounds folder: {}", sounds.display());
            std::fs::create_dir_all(&sounds)?;
            let settings_path = data.join("settings.toml");
            let settings = Settings::load(&settings_path);

            let engine = Engine::start(sounds).map(|(engine, events)| {
                let engine = Arc::new(engine);
                forward_events(app.handle().clone(), events);
                engine.set_volume_db(settings.volume_db);
                engine.set_touch(settings.touch);
                restore_instrument(engine.clone(), settings.instrument.clone());
                engine
            });
            if let Err(e) = &engine {
                eprintln!("audio engine failed to start: {e}");
            }
            app.manage(Pulsar {
                engine,
                settings: Mutex::new(settings),
                settings_path,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            instruments,
            load_instrument,
            note,
            pedal,
            set_volume,
            set_touch,
            set_key_filter,
            set_strips_as_controls,
            open_sounds_folder,
            check_update,
            install_update,
            songs::list_songs,
            songs::get_song,
            songs::import_songs_dialog,
            songs::import_song_files,
            songs::get_takes,
            songs::save_takes,
            songs::rename_song,
            songs::save_song,
            songs::delete_song
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
