use serde::{Deserialize, Serialize};

const MAX_SCOREBOARD_ENTRIES: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScoreEntry {
    pub timestamp_secs: u64,
    pub correct: u32,
    pub incorrect: u32,
    pub elapsed_secs: u64,
    pub points: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScoreBoard {
    pub entries: Vec<ScoreEntry>,
}

impl ScoreBoard {
    pub fn add_entry(&mut self, entry: ScoreEntry) {
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
}
