//! Song library commands. Songs are plain data; practicing happens in the UI.

use std::path::PathBuf;

use pulsar_song::{Library, Song, SongSummary};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

pub struct Songs(pub Library);

/// Imports keep going past bad files, so one broken .mid doesn't block the rest.
#[derive(Serialize)]
pub struct Imported {
    songs: Vec<SongSummary>,
    errors: Vec<String>,
}

fn import_all(library: &Library, paths: impl IntoIterator<Item = PathBuf>) -> Imported {
    let mut imported = Imported {
        songs: Vec::new(),
        errors: Vec::new(),
    };
    for path in paths {
        match library.import(&path) {
            Ok(song) => imported.songs.push(song),
            Err(e) => imported.errors.push(e),
        }
    }
    imported
}

#[tauri::command]
pub fn list_songs(songs: State<Songs>) -> Vec<SongSummary> {
    songs.0.list()
}

#[tauri::command]
pub fn get_song(songs: State<Songs>, id: String) -> Result<Song, String> {
    songs.0.load(&id)
}

/// Opens the native file picker and imports whatever MIDI files are chosen.
#[tauri::command]
pub async fn import_songs_dialog(app: AppHandle) -> Result<Imported, String> {
    let dialog_app = app.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .set_title("Import songs")
            .add_filter("MIDI files", &["mid", "midi"])
            .blocking_pick_files()
    })
    .await
    .map_err(|e| e.to_string())?;
    let paths = picked
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.into_path().ok());
    Ok(import_all(&app.state::<Songs>().0, paths))
}

/// Files dropped onto the window.
#[tauri::command]
pub fn import_song_files(songs: State<Songs>, paths: Vec<PathBuf>) -> Imported {
    import_all(&songs.0, paths)
}

#[tauri::command]
pub fn rename_song(
    songs: State<Songs>,
    id: String,
    title: String,
    artist: String,
) -> Result<SongSummary, String> {
    songs.0.rename(&id, &title, &artist)
}

/// Saves a song from the editor: over `id` if given, else as a new song.
#[tauri::command]
pub fn save_song(
    songs: State<Songs>,
    id: Option<String>,
    song: Song,
) -> Result<SongSummary, String> {
    let song = song.normalized();
    match id {
        Some(id) => songs.0.save(&id, &song),
        None => songs.0.create(&song),
    }
}

#[tauri::command]
pub fn delete_song(songs: State<Songs>, id: String) -> Result<(), String> {
    songs.0.delete(&id)
}
