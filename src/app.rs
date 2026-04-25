use rand::seq::SliceRandom;

use crate::kana::{COLUMN_INDEX_GROUPS, COLUMN_LABELS, HIRAGANA_BASIC_46};

pub(crate) enum GameMode {
    Infinite,
    BestOf(u32),
    Progressive,
}

pub(crate) enum RenderStyle {
    Braille,
    Ascii,
}

pub(crate) enum AppState {
    Menu,
    ColumnOptions,
    InProgress,
    ShowingFeedback,
    ColumnUnlocked,
    Finished,
}

pub(crate) struct App {
    pub(crate) running: bool,
    pub(crate) state: AppState,
    pub(crate) mode: GameMode,
    pub(crate) render_style: RenderStyle,
    pub(crate) menu_selection: usize,
    pub(crate) selected_columns: [bool; 10],
    pub(crate) options_selection: usize,
    pub(crate) options_feedback: Option<String>,
    pub(crate) input: String,
    pub(crate) correct: u32,
    pub(crate) incorrect: u32,
    pub(crate) last_feedback: Option<String>,
    pub(crate) last_correct: Option<bool>,
    pub(crate) deck: Vec<usize>,
    pub(crate) deck_position: usize,
    pub(crate) current_index: usize,
    pub(crate) kana_correct_counts: [u32; 46],
    pub(crate) progressive_unlocked_columns: usize,
    pub(crate) streak: u32,
    pub(crate) max_streak: u32,
    pub(crate) column_attempts: [u32; 10],
    pub(crate) column_correct: [u32; 10],
    pub(crate) questions_to_unlock: [Option<u32>; 10],
    pub(crate) newly_unlocked_column: Option<usize>,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            running: true,
            state: AppState::Menu,
            mode: GameMode::Infinite,
            render_style: RenderStyle::Ascii,
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
        }
    }

    pub(crate) fn refill_deck(&mut self) {
        self.deck = self.allowed_indices();
        self.deck.shuffle(&mut rand::rng());
        self.deck_position = 0;
    }

    pub(crate) fn allowed_indices(&self) -> Vec<usize> {
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

    pub(crate) fn expected_romaji(&self) -> &str {
        HIRAGANA_BASIC_46[self.current_index].1
    }

    pub(crate) fn current_hiragana(&self) -> &str {
        HIRAGANA_BASIC_46[self.current_index].0
    }

    pub(crate) fn advance_prompt(&mut self) {
        if self.deck_position >= self.deck.len() {
            self.refill_deck();
        }

        self.current_index = self.deck[self.deck_position];
        self.deck_position += 1;
    }

    pub(crate) fn evaluate_current_answer(&mut self) {
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
            self.last_feedback = Some(format!("Correct: {} → {}", shown, expected));
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
            self.state = AppState::Finished;
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
                self.state = AppState::ColumnUnlocked;
                return;
            }

            self.state = AppState::Finished;
            return;
        }

        self.state = AppState::ShowingFeedback;
    }

    pub(crate) fn reached_mode_limit(&self) -> bool {
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

    pub(crate) fn select_mode(&self) -> GameMode {
        if self.menu_selection == 0 {
            return GameMode::Infinite;
        }
        if self.menu_selection == 1 {
            return GameMode::BestOf(20);
        }
        if self.menu_selection == 2 {
            return GameMode::Progressive;
        }

        GameMode::Infinite
    }

    pub(crate) fn render_style_label(&self) -> &'static str {
        match self.render_style {
            RenderStyle::Braille => "Braille",
            RenderStyle::Ascii => "Ascii",
        }
    }

    pub(crate) fn toggle_render_style(&mut self) {
        self.render_style = match self.render_style {
            RenderStyle::Braille => RenderStyle::Ascii,
            RenderStyle::Ascii => RenderStyle::Braille,
        };
    }

    pub(crate) fn prepare_selected_mode(&mut self) {
        self.mode = self.select_mode();
        if matches!(self.mode, GameMode::Progressive) {
            self.start_progressive_mode();
            return;
        }
        self.options_selection = 0;
        self.options_feedback = None;
        self.state = AppState::ColumnOptions;
    }

    pub(crate) fn start_progressive_mode(&mut self) {
        self.state = AppState::InProgress;
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
        self.refill_deck();
        if self.deck.is_empty() {
            self.state = AppState::Menu;
            return;
        }
        self.current_index = self.deck[0];
        self.deck_position = 1;
    }

    pub(crate) fn start_selected_mode(&mut self) {
        self.state = AppState::InProgress;
        self.input.clear();
        self.correct = 0;
        self.incorrect = 0;
        self.last_feedback = None;
        self.last_correct = None;
        self.refill_deck();
        if self.deck.is_empty() {
            self.options_feedback = Some("Enable at least one column to start".to_string());
            self.state = AppState::ColumnOptions;
            return;
        }
        self.current_index = self.deck[0];
        self.deck_position = 1;
    }

    pub(crate) fn accuracy(&self) -> f64 {
        let total = self.correct + self.incorrect;
        if total == 0 {
            return 0.0;
        }

        (self.correct as f64 / total as f64) * 100.0
    }

    pub(crate) fn column_of(&self, index: usize) -> usize {
        COLUMN_INDEX_GROUPS
            .iter()
            .position(|group| group.contains(&index))
            .unwrap_or(0)
    }

    pub(crate) fn is_column_mastered(&self, column: usize) -> bool {
        COLUMN_INDEX_GROUPS[column]
            .iter()
            .all(|index| self.kana_correct_counts[*index] >= 3)
    }

    pub(crate) fn column_progress(&self, column: usize) -> u32 {
        COLUMN_INDEX_GROUPS[column]
            .iter()
            .map(|index| self.kana_correct_counts[*index].min(3))
            .sum()
    }

    pub(crate) fn hardest_column(&self) -> Option<usize> {
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
