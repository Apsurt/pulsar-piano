//! The song library: one JSON file per song in a folder (the app data dir).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::{Song, from_midi};

/// What the library list shows, without loading every note.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SongSummary {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub notes: usize,
    pub seconds: f64,
    pub tracks: usize,
    /// Your best full pass, 0–100, if you've played one.
    pub best: Option<u32>,
    /// Last change, in Unix milliseconds.
    pub modified: u64,
}

pub struct Library {
    dir: PathBuf,
}

impl Library {
    pub fn open(dir: PathBuf) -> std::io::Result<Self> {
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    /// Newest first. Files that fail to load are skipped, not fatal.
    pub fn list(&self) -> Vec<SongSummary> {
        let mut songs: Vec<SongSummary> = fs::read_dir(&self.dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let id = path.file_stem()?.to_str()?.to_owned();
                if path.extension()? != "json" || valid_id(&id).is_err() {
                    return None;
                }
                match self.load(&id) {
                    Ok(song) => {
                        let best = self.best_score(&id);
                        Some(summarize(id, &song, &path, best))
                    }
                    Err(e) => {
                        eprintln!("skipping {}: {e}", path.display());
                        None
                    }
                }
            })
            .collect();
        songs.sort_by_key(|s| std::cmp::Reverse(s.modified));
        songs
    }

    pub fn load(&self, id: &str) -> Result<Song, String> {
        let text = fs::read_to_string(self.path(id)?).map_err(|e| e.to_string())?;
        Song::from_json(&text)
    }

    /// Writes atomically (temp file, then rename) so a crash never leaves half a song.
    pub fn save(&self, id: &str, song: &Song) -> Result<SongSummary, String> {
        let path = self.path(id)?;
        let tmp = path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(song).map_err(|e| e.to_string())?;
        fs::write(&tmp, text).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
        Ok(summarize(id.to_owned(), song, &path, self.best_score(id)))
    }

    /// The overall score of your best take, if there is one.
    fn best_score(&self, id: &str) -> Option<u32> {
        let takes = self.load_takes(id).ok()??;
        let score = takes.pointer("/best/score/overall")?.as_f64()?;
        Some(score.round().clamp(0.0, 100.0) as u32)
    }

    /// Adds a song from a file on disk. Only MIDI for now.
    pub fn import(&self, file: &Path) -> Result<SongSummary, String> {
        let extension = file
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or_default()
            .to_lowercase();
        if extension != "mid" && extension != "midi" {
            return Err(format!(
                "{} isn't a MIDI file: Pulsar imports .mid and .midi for now",
                file.display()
            ));
        }
        let bytes = fs::read(file).map_err(|e| format!("{}: {e}", file.display()))?;
        let title = pretty_title(
            file.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled"),
        );
        let song = from_midi(&bytes, &title)?;
        self.create(&song)
    }

    /// Stores a new song (e.g. made in the editor) under a fresh id derived from its title.
    pub fn create(&self, song: &Song) -> Result<SongSummary, String> {
        self.save(&self.new_id(&song.title), song)
    }

    pub fn rename(&self, id: &str, title: &str, artist: &str) -> Result<SongSummary, String> {
        let mut song = self.load(id)?;
        song.title = title.trim().to_owned();
        song.artist = artist.trim().to_owned();
        self.save(id, &song)
    }

    /// Moves the song (and its takes) into a `.deleted` folder rather than erasing it.
    pub fn delete(&self, id: &str) -> Result<(), String> {
        let path = self.path(id)?;
        let trash = self.dir.join(".deleted");
        fs::create_dir_all(&trash).map_err(|e| e.to_string())?;
        fs::rename(&path, trash.join(format!("{id}.json"))).map_err(|e| e.to_string())?;
        let takes = self.takes_path(id)?;
        if takes.exists() {
            fs::rename(&takes, trash.join(format!("{id}.takes.json")))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Your recorded takes of a song (the app decides what's in them), if any.
    pub fn load_takes(&self, id: &str) -> Result<Option<serde_json::Value>, String> {
        let path = self.takes_path(id)?;
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| e.to_string())
    }

    /// Kept beside the songs in a `.takes` folder, written atomically like songs.
    pub fn save_takes(&self, id: &str, takes: &serde_json::Value) -> Result<(), String> {
        let path = self.takes_path(id)?;
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, takes.to_string()).map_err(|e| e.to_string())?;
        fs::rename(&tmp, &path).map_err(|e| e.to_string())
    }

    fn takes_path(&self, id: &str) -> Result<PathBuf, String> {
        valid_id(id)?;
        Ok(self.dir.join(".takes").join(format!("{id}.json")))
    }

    fn path(&self, id: &str) -> Result<PathBuf, String> {
        valid_id(id)?;
        Ok(self.dir.join(format!("{id}.json")))
    }

    fn new_id(&self, title: &str) -> String {
        let base = slug(title);
        let mut id = base.clone();
        let mut n = 2;
        while self.dir.join(format!("{id}.json")).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        id
    }
}

/// Ids come from the UI, so they must never be able to point outside the library folder.
fn valid_id(id: &str) -> Result<(), String> {
    let ok = !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(format!("invalid song id {id:?}"))
    }
}

fn slug(title: &str) -> String {
    let mut slug = String::new();
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug: String = slug.trim_end_matches('-').chars().take(60).collect();
    if slug.is_empty() { "song".into() } else { slug }
}

/// "helpless_piano-solo" -> "helpless piano solo"
fn pretty_title(stem: &str) -> String {
    let title = stem.replace(['_', '-'], " ");
    let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.is_empty() {
        "Untitled".into()
    } else {
        title
    }
}

fn summarize(id: String, song: &Song, path: &Path, best: Option<u32>) -> SongSummary {
    let modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as u64);
    SongSummary {
        id,
        title: song.title.clone(),
        artist: song.artist.clone(),
        notes: song.notes.len(),
        seconds: song.seconds_at(song.end_beat()),
        tracks: song.tracks.len(),
        best,
        modified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::tests::{on, smf};

    fn temp_library(name: &str) -> (Library, PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("pulsar-song-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        (Library::open(dir.join("songs")).unwrap(), dir)
    }

    #[test]
    fn import_list_rename_delete() {
        let (library, dir) = temp_library("crud");
        let file = dir.join("My_Song-piano.mid");
        fs::write(&file, smf(vec![vec![on(0, 0, 60, 100), on(960, 0, 60, 0)]])).unwrap();

        let imported = library.import(&file).unwrap();
        assert_eq!(
            (imported.id.as_str(), imported.title.as_str()),
            ("my-song-piano", "My Song piano")
        );
        assert_eq!((imported.notes, imported.seconds), (1, 1.0));
        let second = library.import(&file).unwrap();
        assert_eq!(second.id, "my-song-piano-2");
        assert_eq!(library.list().len(), 2);

        let renamed = library
            .rename(&imported.id, " Helpless ", "Hamilton")
            .unwrap();
        assert_eq!(
            (renamed.title.as_str(), renamed.artist.as_str()),
            ("Helpless", "Hamilton")
        );

        // Takes live beside the songs, aren't songs themselves, and go to the trash with their song.
        let takes = serde_json::json!({ "best": { "score": { "overall": 83.6 } } });
        assert_eq!(library.load_takes(&second.id).unwrap(), None);
        library.save_takes(&second.id, &takes).unwrap();
        assert_eq!(library.load_takes(&second.id).unwrap(), Some(takes));
        let list = library.list();
        assert_eq!(list.len(), 2);
        let best = |id: &str| list.iter().find(|s| s.id == id).unwrap().best;
        assert_eq!((best(&second.id), best(&imported.id)), (Some(84), None));

        library.delete(&second.id).unwrap();
        assert_eq!(library.list().len(), 1);
        assert!(dir.join("songs/.deleted/my-song-piano-2.json").exists());
        assert!(
            dir.join("songs/.deleted/my-song-piano-2.takes.json")
                .exists()
        );
        assert!(
            library
                .save_takes("../x", &serde_json::Value::Null)
                .is_err()
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_unsafe_ids_and_non_midi_files() {
        let (library, dir) = temp_library("unsafe");
        assert!(library.load("../../etc/passwd").is_err());
        assert!(library.delete("A").is_err());
        assert!(library.import(Path::new("song.pdf")).is_err());
        let _ = fs::remove_dir_all(&dir);
    }
}
