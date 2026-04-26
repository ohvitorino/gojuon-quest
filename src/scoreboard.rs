use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

const MAX_SCOREBOARD_ENTRIES: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct ScoreEntry {
    pub(crate) timestamp_secs: u64,
    pub(crate) correct: u32,
    pub(crate) incorrect: u32,
    pub(crate) elapsed_secs: u64,
    pub(crate) points: i64,
}

impl ScoreEntry {
    pub(crate) fn now(correct: u32, incorrect: u32, elapsed_secs: u64, points: i64) -> Self {
        let timestamp_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        Self {
            timestamp_secs,
            correct,
            incorrect,
            elapsed_secs,
            points,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct ScoreBoard {
    pub(crate) entries: Vec<ScoreEntry>,
}

impl ScoreBoard {
    pub(crate) fn load() -> Self {
        Self::load_from_path(&storage_path()).unwrap_or_default()
    }

    pub(crate) fn save(&self) -> std::io::Result<()> {
        self.save_to_path(&storage_path())
    }

    pub(crate) fn add_entry(&mut self, entry: ScoreEntry) {
        self.entries.push(entry);
        self.entries.sort_by(|left, right| {
            right
                .points
                .cmp(&left.points)
                .then_with(|| left.elapsed_secs.cmp(&right.elapsed_secs))
                .then_with(|| right.timestamp_secs.cmp(&left.timestamp_secs))
        });
        self.entries.truncate(MAX_SCOREBOARD_ENTRIES);
    }

    fn load_from_path(path: &Path) -> Option<Self> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn save_to_path(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)
    }
}

fn storage_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home)
        .join(".gojuon-quest")
        .join("scoreboard.json")
}
