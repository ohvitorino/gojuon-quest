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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_scoreboard_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        std::env::temp_dir().join(format!("gojuon-quest-{name}-{nanos}.json"))
    }

    #[test]
    fn add_entry_sorts_by_points_then_time() {
        let mut scoreboard = ScoreBoard::default();
        scoreboard.add_entry(ScoreEntry {
            timestamp_secs: 1,
            correct: 18,
            incorrect: 2,
            elapsed_secs: 90,
            points: 1650,
        });
        scoreboard.add_entry(ScoreEntry {
            timestamp_secs: 2,
            correct: 19,
            incorrect: 1,
            elapsed_secs: 110,
            points: 1765,
        });
        scoreboard.add_entry(ScoreEntry {
            timestamp_secs: 3,
            correct: 19,
            incorrect: 1,
            elapsed_secs: 100,
            points: 1765,
        });

        assert_eq!(scoreboard.entries[0].elapsed_secs, 100);
        assert_eq!(scoreboard.entries[1].elapsed_secs, 110);
        assert_eq!(scoreboard.entries[2].points, 1650);
    }

    #[test]
    fn save_and_load_round_trip() {
        let path = temp_scoreboard_path("round-trip");
        let mut scoreboard = ScoreBoard::default();
        scoreboard.add_entry(ScoreEntry {
            timestamp_secs: 10,
            correct: 20,
            incorrect: 0,
            elapsed_secs: 80,
            points: 1920,
        });

        scoreboard
            .save_to_path(&path)
            .expect("scoreboard should save to temp path");
        let loaded = ScoreBoard::load_from_path(&path).expect("scoreboard should load");
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.entries[0].points, 1920);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_corrupt_file_returns_none() {
        let path = temp_scoreboard_path("corrupt");
        fs::write(&path, "{ this is not valid json").expect("corrupt fixture should be writable");
        assert!(ScoreBoard::load_from_path(&path).is_none());
        let _ = fs::remove_file(path);
    }
}
