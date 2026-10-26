//! User settings, stored as TOML in the app data dir (never next to the binary).

use std::path::Path;

use pulsar_engine::Touch;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Id of the last instrument played, restored on launch.
    pub instrument: Option<String>,
    pub volume_db: f32,
    pub touch: Touch,
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) {
        let result = toml::to_string(self)
            .map_err(|e| e.to_string())
            .and_then(|text| std::fs::write(path, text).map_err(|e| e.to_string()));
        if let Err(e) = result {
            eprintln!("could not save settings to {}: {e}", path.display());
        }
    }
}
