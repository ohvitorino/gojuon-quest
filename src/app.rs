use rand::seq::SliceRandom;
use std::time::Instant;

use crate::kana::{COLUMN_INDEX_GROUPS, COLUMN_LABELS, HIRAGANA_BASIC_46};
use crate::scoreboard::ScoreBoard;

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
    pub(crate) session_started_at: Option<Instant>,
    pub(crate) session_elapsed_secs: u64,
    pub(crate) scoreboard: ScoreBoard,
    pub(crate) recorded_score_for_session: bool,
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
            session_started_at: None,
            session_elapsed_secs: 0,
            scoreboard: ScoreBoard::default(),
            recorded_score_for_session: false,
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
            self.update_session_timer();
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
        self.mode = GameMode::Progressive;
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
        self.session_started_at = None;
        self.session_elapsed_secs = 0;
        self.recorded_score_for_session = false;
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
        self.streak = 0;
        self.max_streak = 0;
        self.column_attempts = [0; 10];
        self.column_correct = [0; 10];
        self.session_started_at = None;
        self.session_elapsed_secs = 0;
        self.recorded_score_for_session = false;
        self.last_feedback = None;
        self.last_correct = None;
        if matches!(self.mode, GameMode::BestOf(_)) {
            self.session_started_at = Some(Instant::now());
        }
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

    pub(crate) fn update_session_timer(&mut self) {
        if !matches!(self.mode, GameMode::BestOf(_)) {
            return;
        }
        if !matches!(self.state, AppState::InProgress | AppState::ShowingFeedback) {
            return;
        }
        let Some(started_at) = self.session_started_at else {
            return;
        };
        self.session_elapsed_secs = started_at.elapsed().as_secs();
    }

    pub(crate) fn best_of_points(&self) -> i64 {
        (self.correct as i64 * 100)
            - (self.incorrect as i64 * 25)
            - (self.session_elapsed_secs as i64)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn app_for_answer(index: usize, answer: &str) -> App {
        let mut app = App::new();
        app.current_index = index;
        app.input = answer.to_string();
        app
    }

    #[test]
    fn new_app_defaults() {
        let app = App::new();
        assert!(app.running);
        assert!(matches!(app.state, AppState::Menu));
        assert!(matches!(app.mode, GameMode::Infinite));
        assert!(matches!(app.render_style, RenderStyle::Ascii));
        assert_eq!(app.correct, 0);
        assert_eq!(app.incorrect, 0);
        assert_eq!(app.streak, 0);
        assert_eq!(app.max_streak, 0);
        assert!(app.input.is_empty());
        assert!(app.deck.is_empty());
    }

    #[test]
    fn allowed_indices_all_columns_returns_all_46() {
        let app = App::new();
        assert_eq!(app.allowed_indices().len(), 46);
    }

    #[test]
    fn allowed_indices_no_columns_returns_empty() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        assert!(app.allowed_indices().is_empty());
    }

    #[test]
    fn allowed_indices_single_column_returns_that_group() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.selected_columns[0] = true;
        assert_eq!(app.allowed_indices(), vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn allowed_indices_progressive_returns_first_n_groups() {
        let mut app = App::new();
        app.mode = GameMode::Progressive;
        app.progressive_unlocked_columns = 2;
        let indices = app.allowed_indices();
        let expected: Vec<usize> = (0..10).collect();
        assert_eq!(indices, expected);
    }

    #[test]
    fn column_of_maps_indices_to_correct_groups() {
        let app = App::new();
        assert_eq!(app.column_of(0), 0);
        assert_eq!(app.column_of(4), 0);
        assert_eq!(app.column_of(5), 1);
        assert_eq!(app.column_of(45), 9);
    }

    #[test]
    fn accuracy_is_zero_with_no_answers() {
        let app = App::new();
        assert_eq!(app.accuracy(), 0.0);
    }

    #[test]
    fn accuracy_calculates_correct_percentage() {
        let mut app = App::new();
        app.correct = 3;
        app.incorrect = 1;
        assert_eq!(app.accuracy(), 75.0);
    }

    #[test]
    fn accuracy_at_100_percent() {
        let mut app = App::new();
        app.correct = 5;
        assert_eq!(app.accuracy(), 100.0);
    }

    #[test]
    fn is_column_mastered_false_when_insufficient() {
        let app = App::new();
        assert!(!app.is_column_mastered(0));
    }

    #[test]
    fn is_column_mastered_true_when_all_counts_at_least_3() {
        let mut app = App::new();
        for i in 0..5 {
            app.kana_correct_counts[i] = 3;
        }
        assert!(app.is_column_mastered(0));
    }

    #[test]
    fn is_column_mastered_false_when_one_short() {
        let mut app = App::new();
        for i in 0..4 {
            app.kana_correct_counts[i] = 3;
        }
        app.kana_correct_counts[4] = 2;
        assert!(!app.is_column_mastered(0));
    }

    #[test]
    fn column_progress_sums_counts_capped_at_3() {
        let mut app = App::new();
        app.kana_correct_counts[0] = 3;
        app.kana_correct_counts[1] = 5; // capped to 3
        app.kana_correct_counts[2] = 1;
        // indices 3, 4 stay at 0
        assert_eq!(app.column_progress(0), 7);
    }

    #[test]
    fn reached_mode_limit_infinite_never() {
        let mut app = App::new();
        app.correct = 1_000_000;
        assert!(!app.reached_mode_limit());
    }

    #[test]
    fn reached_mode_limit_best_of_triggers_at_limit() {
        let mut app = App::new();
        app.mode = GameMode::BestOf(20);
        app.correct = 15;
        app.incorrect = 5;
        assert!(app.reached_mode_limit());
    }

    #[test]
    fn reached_mode_limit_best_of_not_triggered_before_limit() {
        let mut app = App::new();
        app.mode = GameMode::BestOf(20);
        app.correct = 10;
        app.incorrect = 5;
        assert!(!app.reached_mode_limit());
    }

    #[test]
    fn evaluate_correct_answer_increments_stats() {
        let mut app = app_for_answer(0, "a");
        app.evaluate_current_answer();
        assert_eq!(app.correct, 1);
        assert_eq!(app.incorrect, 0);
        assert_eq!(app.streak, 1);
        assert_eq!(app.max_streak, 1);
        assert_eq!(app.kana_correct_counts[0], 1);
        assert_eq!(app.last_correct, Some(true));
        assert!(app.input.is_empty());
        assert!(matches!(app.state, AppState::ShowingFeedback));
    }

    #[test]
    fn evaluate_incorrect_answer_increments_stats() {
        let mut app = app_for_answer(0, "wrong");
        app.evaluate_current_answer();
        assert_eq!(app.correct, 0);
        assert_eq!(app.incorrect, 1);
        assert_eq!(app.streak, 0);
        assert_eq!(app.last_correct, Some(false));
        assert!(matches!(app.state, AppState::ShowingFeedback));
    }

    #[test]
    fn evaluate_trims_and_lowercases_input() {
        let mut app = app_for_answer(0, "  A  ");
        app.evaluate_current_answer();
        assert_eq!(app.correct, 1);
    }

    #[test]
    fn streak_resets_on_incorrect() {
        let mut app = app_for_answer(0, "a");
        app.evaluate_current_answer();
        assert_eq!(app.streak, 1);

        app.current_index = 0;
        app.input = "wrong".to_string();
        app.evaluate_current_answer();
        assert_eq!(app.streak, 0);
        assert_eq!(app.max_streak, 1);
    }

    #[test]
    fn max_streak_tracks_peak() {
        let mut app = App::new();
        for _ in 0..3 {
            app.current_index = 0;
            app.input = "a".to_string();
            app.evaluate_current_answer();
        }
        assert_eq!(app.streak, 3);
        assert_eq!(app.max_streak, 3);

        app.current_index = 0;
        app.input = "wrong".to_string();
        app.evaluate_current_answer();
        assert_eq!(app.streak, 0);
        assert_eq!(app.max_streak, 3);
    }

    #[test]
    fn evaluate_triggers_finished_when_best_of_limit_reached() {
        let mut app = app_for_answer(0, "a");
        app.mode = GameMode::BestOf(1);
        app.evaluate_current_answer();
        assert!(matches!(app.state, AppState::Finished));
    }

    #[test]
    fn kana_correct_count_caps_at_3() {
        let mut app = app_for_answer(0, "a");
        app.kana_correct_counts[0] = 3;
        app.evaluate_current_answer();
        assert_eq!(app.kana_correct_counts[0], 3);
    }

    #[test]
    fn refill_deck_contains_all_allowed_indices() {
        let mut app = App::new();
        app.refill_deck();
        let mut deck = app.deck.clone();
        deck.sort_unstable();
        assert_eq!(deck, (0..46).collect::<Vec<_>>());
        assert_eq!(app.deck_position, 0);
    }

    #[test]
    fn refill_deck_single_column_contains_only_that_group() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.selected_columns[1] = true;
        app.refill_deck();
        let mut deck = app.deck.clone();
        deck.sort_unstable();
        assert_eq!(deck, vec![5, 6, 7, 8, 9]);
    }

    #[test]
    fn advance_prompt_sets_current_index_and_increments_position() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.selected_columns[0] = true;
        app.refill_deck();
        let first = app.deck[0];
        app.advance_prompt();
        assert_eq!(app.current_index, first);
        assert_eq!(app.deck_position, 1);
    }

    #[test]
    fn advance_prompt_refills_when_deck_exhausted() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.selected_columns[0] = true;
        app.refill_deck();
        app.deck_position = 5;
        app.advance_prompt();
        assert_eq!(app.deck_position, 1);
        assert_eq!(app.deck.len(), 5);
    }

    #[test]
    fn toggle_render_style_switches_between_ascii_and_braille() {
        let mut app = App::new();
        assert_eq!(app.render_style_label(), "Ascii");
        app.toggle_render_style();
        assert_eq!(app.render_style_label(), "Braille");
        app.toggle_render_style();
        assert_eq!(app.render_style_label(), "Ascii");
    }

    #[test]
    fn select_mode_maps_menu_selections() {
        let mut app = App::new();
        app.menu_selection = 0;
        assert!(matches!(app.select_mode(), GameMode::Infinite));
        app.menu_selection = 1;
        assert!(matches!(app.select_mode(), GameMode::BestOf(20)));
        app.menu_selection = 2;
        assert!(matches!(app.select_mode(), GameMode::Progressive));
        app.menu_selection = 99;
        assert!(matches!(app.select_mode(), GameMode::Infinite));
    }

    #[test]
    fn hardest_column_none_when_no_attempts() {
        let app = App::new();
        assert!(app.hardest_column().is_none());
    }

    #[test]
    fn hardest_column_single_column_with_attempts() {
        let mut app = App::new();
        app.column_attempts[2] = 5;
        app.column_correct[2] = 2;
        assert_eq!(app.hardest_column(), Some(2));
    }

    #[test]
    fn hardest_column_returns_lowest_accuracy_column() {
        let mut app = App::new();
        app.column_attempts[0] = 5;
        app.column_correct[0] = 4; // 80%
        app.column_attempts[1] = 5;
        app.column_correct[1] = 1; // 20%
        assert_eq!(app.hardest_column(), Some(1));
    }

    #[test]
    fn start_selected_mode_resets_and_enters_in_progress() {
        let mut app = App::new();
        app.correct = 10;
        app.incorrect = 5;
        app.streak = 3;
        app.max_streak = 8;
        app.start_selected_mode();
        assert_eq!(app.correct, 0);
        assert_eq!(app.incorrect, 0);
        assert_eq!(app.streak, 0);
        assert_eq!(app.max_streak, 0);
        assert!(app.input.is_empty());
        assert!(matches!(app.state, AppState::InProgress));
        assert!(!app.deck.is_empty());
    }

    #[test]
    fn start_selected_mode_empty_deck_goes_to_column_options() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.start_selected_mode();
        assert!(matches!(app.state, AppState::ColumnOptions));
        assert!(app.options_feedback.is_some());
    }

    #[test]
    fn start_progressive_mode_unlocks_first_column_only() {
        let mut app = App::new();
        app.start_progressive_mode();
        assert_eq!(app.progressive_unlocked_columns, 1);
        assert!(matches!(app.state, AppState::InProgress));
        assert!(app.current_index < 5);
    }

    #[test]
    fn start_selected_mode_best_of_starts_session_timer() {
        let mut app = App::new();
        app.mode = GameMode::BestOf(20);
        app.start_selected_mode();
        assert!(app.session_started_at.is_some());
    }

    #[test]
    fn best_of_points_uses_weighted_formula() {
        let mut app = App::new();
        app.correct = 15;
        app.incorrect = 5;
        app.session_elapsed_secs = 42;
        assert_eq!(app.best_of_points(), 1333);
    }

    #[test]
    fn update_session_timer_does_not_tick_when_not_in_active_best_of_state() {
        let mut app = App::new();
        app.mode = GameMode::BestOf(20);
        app.state = AppState::Finished;
        app.session_started_at = Some(Instant::now());
        app.session_elapsed_secs = 7;
        app.update_session_timer();
        assert_eq!(app.session_elapsed_secs, 7);
    }
}
