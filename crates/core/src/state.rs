use rand::seq::SliceRandom;

use crate::kana::{COLUMN_INDEX_GROUPS, COLUMN_LABELS, HIRAGANA_BASIC_46};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Infinite,
    BestOf(u32),
    Progressive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameStateKind {
    Menu,
    ColumnOptions,
    InProgress,
    ShowingFeedback,
    ColumnUnlocked,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuitPrompt {
    ExitApplication,
    AbandonSession,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub running: bool,
    pub state: GameStateKind,
    pub quit_prompt: Option<QuitPrompt>,
    pub mode: GameMode,
    pub menu_selection: usize,
    pub selected_columns: [bool; 10],
    pub options_selection: usize,
    pub options_feedback: Option<String>,
    pub input: String,
    pub correct: u32,
    pub incorrect: u32,
    pub last_feedback: Option<String>,
    pub last_correct: Option<bool>,
    pub deck: Vec<usize>,
    pub deck_position: usize,
    pub current_index: usize,
    pub kana_correct_counts: [u32; 46],
    pub progressive_unlocked_columns: usize,
    pub streak: u32,
    pub max_streak: u32,
    pub column_attempts: [u32; 10],
    pub column_correct: [u32; 10],
    pub questions_to_unlock: [Option<u32>; 10],
    pub newly_unlocked_column: Option<usize>,
    pub session_elapsed_secs: u64,
    pub recorded_score_for_session: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            running: true,
            state: GameStateKind::Menu,
            quit_prompt: None,
            mode: GameMode::Infinite,
            menu_selection: 0,
            selected_columns: [true; 10],
            options_selection: 0,
            options_feedback: None,
            input: String::new(),
            correct: 0,
            incorrect: 0,
            last_feedback: None,
            last_correct: None,
            deck: Vec::new(),
            deck_position: 0,
            current_index: 0,
            kana_correct_counts: [0; 46],
            progressive_unlocked_columns: 1,
            streak: 0,
            max_streak: 0,
            column_attempts: [0; 10],
            column_correct: [0; 10],
            questions_to_unlock: [None; 10],
            newly_unlocked_column: None,
            session_elapsed_secs: 0,
            recorded_score_for_session: false,
        }
    }
}

impl GameState {
    pub fn refill_deck(&mut self) {
        self.deck = self.allowed_indices();
        self.deck.shuffle(&mut rand::rng());
        self.deck_position = 0;
    }

    pub fn allowed_indices(&self) -> Vec<usize> {
        if matches!(self.mode, GameMode::Progressive) {
            return COLUMN_INDEX_GROUPS
                .iter()
                .take(self.progressive_unlocked_columns)
                .flat_map(|group| group.iter().copied())
                .collect();
        }

        let mut indices = Vec::new();
        for (column, enabled) in self.selected_columns.iter().enumerate() {
            if !enabled {
                continue;
            }
            indices.extend_from_slice(COLUMN_INDEX_GROUPS[column]);
        }
        indices
    }

    pub fn expected_romaji(&self) -> &str {
        HIRAGANA_BASIC_46[self.current_index].1
    }

    pub fn current_hiragana(&self) -> &str {
        HIRAGANA_BASIC_46[self.current_index].0
    }

    pub fn advance_prompt(&mut self) {
        if self.deck_position >= self.deck.len() {
            self.refill_deck();
        }
        self.current_index = self.deck[self.deck_position];
        self.deck_position += 1;
    }

    pub fn evaluate_current_answer(&mut self) {
        let expected = self.expected_romaji().to_string();
        let shown = self.current_hiragana().to_string();
        let typed = self.input.trim().to_ascii_lowercase();
        let is_correct = typed == expected;
        let answered_column = self.column_of(self.current_index);

        self.column_attempts[answered_column] += 1;
        if is_correct {
            self.correct += 1;
            self.column_correct[answered_column] += 1;
            self.kana_correct_counts[self.current_index] =
                (self.kana_correct_counts[self.current_index] + 1).min(3);
            self.streak += 1;
            self.max_streak = self.max_streak.max(self.streak);
            self.last_feedback = Some(format!("Correct: {} -> {}", shown, expected));
        } else {
            self.incorrect += 1;
            self.streak = 0;
            self.last_feedback = Some(format!(
                "Incorrect: {} expected '{}', got '{}'",
                shown, expected, typed
            ));
        }

        self.last_correct = Some(is_correct);
        self.input.clear();

        if self.reached_mode_limit() {
            self.state = GameStateKind::Finished;
            return;
        }

        if matches!(self.mode, GameMode::Progressive)
            && self.progressive_unlocked_columns > 0
            && self.is_column_mastered(self.progressive_unlocked_columns - 1)
        {
            let mastered_column = self.progressive_unlocked_columns - 1;
            self.questions_to_unlock[mastered_column] = Some(self.correct + self.incorrect);
            if self.progressive_unlocked_columns < COLUMN_LABELS.len() {
                self.newly_unlocked_column = Some(self.progressive_unlocked_columns);
                self.progressive_unlocked_columns += 1;
                self.refill_deck();
                self.state = GameStateKind::ColumnUnlocked;
                return;
            }

            self.state = GameStateKind::Finished;
            return;
        }

        self.state = GameStateKind::ShowingFeedback;
    }

    pub fn reached_mode_limit(&self) -> bool {
        let answered = self.correct + self.incorrect;
        match self.mode {
            GameMode::Infinite => false,
            GameMode::BestOf(limit) => answered >= limit,
            GameMode::Progressive => {
                self.progressive_unlocked_columns >= COLUMN_LABELS.len()
                    && self.is_column_mastered(COLUMN_LABELS.len() - 1)
            }
        }
    }

    pub fn select_mode(&self) -> GameMode {
        match self.menu_selection {
            0 => GameMode::Progressive,
            1 => GameMode::BestOf(20),
            2 => GameMode::Infinite,
            _ => GameMode::Infinite,
        }
    }

    pub fn prepare_selected_mode(&mut self) {
        self.mode = self.select_mode();
        if matches!(self.mode, GameMode::Progressive) {
            self.start_progressive_mode();
            return;
        }
        self.options_selection = 0;
        self.options_feedback = None;
        self.state = GameStateKind::ColumnOptions;
    }

    pub fn start_progressive_mode(&mut self) {
        self.mode = GameMode::Progressive;
        self.state = GameStateKind::InProgress;
        self.input.clear();
        self.correct = 0;
        self.incorrect = 0;
        self.last_feedback = None;
        self.last_correct = None;
        self.kana_correct_counts = [0; 46];
        self.progressive_unlocked_columns = 1;
        self.streak = 0;
        self.max_streak = 0;
        self.column_attempts = [0; 10];
        self.column_correct = [0; 10];
        self.questions_to_unlock = [None; 10];
        self.newly_unlocked_column = None;
        self.session_elapsed_secs = 0;
        self.recorded_score_for_session = false;
        self.refill_deck();
        if self.deck.is_empty() {
            self.state = GameStateKind::Menu;
            return;
        }
        self.current_index = self.deck[0];
        self.deck_position = 1;
    }

    pub fn start_selected_mode(&mut self) {
        self.state = GameStateKind::InProgress;
        self.input.clear();
        self.correct = 0;
        self.incorrect = 0;
        self.streak = 0;
        self.max_streak = 0;
        self.column_attempts = [0; 10];
        self.column_correct = [0; 10];
        self.session_elapsed_secs = 0;
        self.recorded_score_for_session = false;
        self.last_feedback = None;
        self.last_correct = None;
        self.refill_deck();
        if self.deck.is_empty() {
            self.options_feedback = Some("Enable at least one column to start".to_string());
            self.state = GameStateKind::ColumnOptions;
            return;
        }
        self.current_index = self.deck[0];
        self.deck_position = 1;
    }

    pub fn accuracy(&self) -> f64 {
        let total = self.correct + self.incorrect;
        if total == 0 {
            return 0.0;
        }
        (self.correct as f64 / total as f64) * 100.0
    }

    pub fn best_of_points(&self) -> i64 {
        (self.correct as i64 * 100)
            - (self.incorrect as i64 * 25)
            - (self.session_elapsed_secs as i64)
    }

    pub fn column_of(&self, index: usize) -> usize {
        COLUMN_INDEX_GROUPS
            .iter()
            .position(|group| group.contains(&index))
            .unwrap_or(0)
    }

    pub fn is_column_mastered(&self, column: usize) -> bool {
        COLUMN_INDEX_GROUPS[column]
            .iter()
            .all(|index| self.kana_correct_counts[*index] >= 3)
    }

    pub fn column_progress(&self, column: usize) -> u32 {
        COLUMN_INDEX_GROUPS[column]
            .iter()
            .map(|index| self.kana_correct_counts[*index].min(3))
            .sum()
    }

    pub fn hardest_column(&self) -> Option<usize> {
        (0..COLUMN_LABELS.len())
            .filter(|column| self.column_attempts[*column] > 0)
            .min_by(|a, b| {
                let left = self.column_correct[*a] as u64 * self.column_attempts[*b] as u64;
                let right = self.column_correct[*b] as u64 * self.column_attempts[*a] as u64;
                left.cmp(&right)
                    .then_with(|| self.column_attempts[*b].cmp(&self.column_attempts[*a]))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accuracy_and_points_follow_expected_rules() {
        let state = GameState {
            correct: 15,
            incorrect: 5,
            session_elapsed_secs: 42,
            ..GameState::default()
        };
        assert_eq!(state.accuracy(), 75.0);
        assert_eq!(state.best_of_points(), 1333);
    }

    #[test]
    fn best_of_reaches_limit() {
        let state = GameState {
            mode: GameMode::BestOf(20),
            correct: 19,
            incorrect: 1,
            ..GameState::default()
        };
        assert!(state.reached_mode_limit());
    }

    #[test]
    fn progressive_unlock_requires_mastery() {
        let mut state = GameState {
            mode: GameMode::Progressive,
            progressive_unlocked_columns: 1,
            ..GameState::default()
        };
        assert!(!state.is_column_mastered(0));
        for idx in 0..5 {
            state.kana_correct_counts[idx] = 3;
        }
        assert!(state.is_column_mastered(0));
    }
}
