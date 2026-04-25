use crossterm::event::KeyCode;

use crate::app::{App, AppState};
use crate::kana::COLUMN_LABELS;

pub(crate) fn handle_in_progress_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.state = AppState::Menu,
        KeyCode::Enter => app.evaluate_current_answer(),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => app.input.push(c),
        _ => {}
    }
}

pub(crate) fn handle_showing_feedback_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.state = AppState::Menu,
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.advance_prompt();
            app.last_feedback = None;
            app.last_correct = None;
            app.state = AppState::InProgress;
        }
        _ => {}
    }
}

pub(crate) fn handle_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.running = false,
        KeyCode::Up | KeyCode::Char('k') => {
            app.menu_selection = app.menu_selection.saturating_sub(1)
        }
        KeyCode::Down | KeyCode::Char('j') => app.menu_selection = (app.menu_selection + 1).min(3),
        KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
            if app.menu_selection == 3 {
                app.toggle_render_style();
            }
        }
        KeyCode::Enter => {
            if app.menu_selection == 3 {
                app.toggle_render_style();
                return;
            }
            app.prepare_selected_mode();
        }
        _ => {}
    }
}

pub(crate) fn handle_column_options_key(app: &mut App, code: KeyCode) {
    let last_row = COLUMN_LABELS.len();
    match code {
        KeyCode::Esc => {
            app.state = AppState::Menu;
            app.options_feedback = None;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.options_selection = app.options_selection.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.options_selection = (app.options_selection + 1).min(last_row);
        }
        KeyCode::Char('s') => {
            if app.allowed_indices().is_empty() {
                app.options_feedback = Some("Enable at least one column to start".to_string());
                return;
            }
            app.options_feedback = None;
            app.start_selected_mode();
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if app.options_selection == last_row {
                if app.allowed_indices().is_empty() {
                    app.options_feedback = Some("Enable at least one column to start".to_string());
                    return;
                }
                app.options_feedback = None;
                app.start_selected_mode();
                return;
            }
            app.selected_columns[app.options_selection] =
                !app.selected_columns[app.options_selection];
            app.options_feedback = None;
        }
        _ => {}
    }
}

pub(crate) fn handle_finished_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.running = false,
        KeyCode::Enter => app.state = AppState::Menu,
        _ => {}
    }
}

pub(crate) fn handle_column_unlocked_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.state = AppState::Menu,
        KeyCode::Enter | KeyCode::Char(' ') => {
            app.newly_unlocked_column = None;
            app.advance_prompt();
            app.last_feedback = None;
            app.last_correct = None;
            app.state = AppState::InProgress;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyCode;

    use super::*;
    use crate::app::{App, AppState};
    use crate::kana::COLUMN_LABELS;

    #[test]
    fn in_progress_esc_returns_to_menu() {
        let mut app = App::new();
        handle_in_progress_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.state, AppState::Menu));
    }

    #[test]
    fn in_progress_char_appends_to_input() {
        let mut app = App::new();
        handle_in_progress_key(&mut app, KeyCode::Char('a'));
        handle_in_progress_key(&mut app, KeyCode::Char('b'));
        assert_eq!(app.input, "ab");
    }

    #[test]
    fn in_progress_backspace_removes_last_char() {
        let mut app = App::new();
        app.input = "abc".to_string();
        handle_in_progress_key(&mut app, KeyCode::Backspace);
        assert_eq!(app.input, "ab");
    }

    #[test]
    fn in_progress_backspace_on_empty_input_is_safe() {
        let mut app = App::new();
        handle_in_progress_key(&mut app, KeyCode::Backspace);
        assert!(app.input.is_empty());
    }

    #[test]
    fn in_progress_enter_evaluates_answer() {
        let mut app = App::new();
        app.current_index = 0; // あ -> "a"
        app.input = "a".to_string();
        handle_in_progress_key(&mut app, KeyCode::Enter);
        assert_eq!(app.correct, 1);
    }

    #[test]
    fn showing_feedback_esc_returns_to_menu() {
        let mut app = App::new();
        handle_showing_feedback_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.state, AppState::Menu));
    }

    #[test]
    fn showing_feedback_enter_advances_to_in_progress() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.selected_columns[0] = true;
        app.refill_deck();
        handle_showing_feedback_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.state, AppState::InProgress));
        assert!(app.last_feedback.is_none());
        assert!(app.last_correct.is_none());
    }

    #[test]
    fn showing_feedback_space_also_advances() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        app.selected_columns[0] = true;
        app.refill_deck();
        handle_showing_feedback_key(&mut app, KeyCode::Char(' '));
        assert!(matches!(app.state, AppState::InProgress));
    }

    #[test]
    fn menu_esc_stops_running() {
        let mut app = App::new();
        handle_menu_key(&mut app, KeyCode::Esc);
        assert!(!app.running);
    }

    #[test]
    fn menu_up_decrements_selection_with_floor_at_zero() {
        let mut app = App::new();
        app.menu_selection = 0;
        handle_menu_key(&mut app, KeyCode::Up);
        assert_eq!(app.menu_selection, 0);

        app.menu_selection = 2;
        handle_menu_key(&mut app, KeyCode::Char('k'));
        assert_eq!(app.menu_selection, 1);
    }

    #[test]
    fn menu_down_increments_selection_capped_at_3() {
        let mut app = App::new();
        app.menu_selection = 3;
        handle_menu_key(&mut app, KeyCode::Down);
        assert_eq!(app.menu_selection, 3);

        app.menu_selection = 1;
        handle_menu_key(&mut app, KeyCode::Char('j'));
        assert_eq!(app.menu_selection, 2);
    }

    #[test]
    fn menu_left_right_only_toggles_render_on_row_3() {
        let mut app = App::new();
        app.menu_selection = 1;
        let label_before = app.render_style_label();
        handle_menu_key(&mut app, KeyCode::Left);
        assert_eq!(app.render_style_label(), label_before);

        app.menu_selection = 3;
        handle_menu_key(&mut app, KeyCode::Right);
        assert_ne!(app.render_style_label(), label_before);
    }

    #[test]
    fn menu_h_l_only_toggles_render_on_row_3() {
        let mut app = App::new();
        app.menu_selection = 0;
        let label_before = app.render_style_label();
        handle_menu_key(&mut app, KeyCode::Char('h'));
        assert_eq!(app.render_style_label(), label_before);

        app.menu_selection = 3;
        handle_menu_key(&mut app, KeyCode::Char('l'));
        assert_ne!(app.render_style_label(), label_before);
    }

    #[test]
    fn menu_enter_on_row_3_toggles_render_style() {
        let mut app = App::new();
        app.menu_selection = 3;
        let label_before = app.render_style_label();
        handle_menu_key(&mut app, KeyCode::Enter);
        assert_ne!(app.render_style_label(), label_before);
    }

    #[test]
    fn menu_enter_on_progressive_mode_starts_progressive() {
        let mut app = App::new();
        app.menu_selection = 0;
        handle_menu_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.state, AppState::InProgress));
    }

    #[test]
    fn menu_enter_on_infinite_mode_goes_to_column_options() {
        let mut app = App::new();
        app.menu_selection = 2;
        handle_menu_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.state, AppState::ColumnOptions));
    }

    #[test]
    fn column_options_esc_returns_to_menu_and_clears_feedback() {
        let mut app = App::new();
        app.options_feedback = Some("error".to_string());
        handle_column_options_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.state, AppState::Menu));
        assert!(app.options_feedback.is_none());
    }

    #[test]
    fn column_options_enter_toggles_selected_column() {
        let mut app = App::new();
        app.options_selection = 0;
        assert!(app.selected_columns[0]);
        handle_column_options_key(&mut app, KeyCode::Enter);
        assert!(!app.selected_columns[0]);
        handle_column_options_key(&mut app, KeyCode::Enter);
        assert!(app.selected_columns[0]);
    }

    #[test]
    fn column_options_space_also_toggles_column() {
        let mut app = App::new();
        app.options_selection = 2;
        let before = app.selected_columns[2];
        handle_column_options_key(&mut app, KeyCode::Char(' '));
        assert_eq!(app.selected_columns[2], !before);
    }

    #[test]
    fn column_options_s_starts_game_when_columns_selected() {
        let mut app = App::new();
        handle_column_options_key(&mut app, KeyCode::Char('s'));
        assert!(matches!(app.state, AppState::InProgress));
    }

    #[test]
    fn column_options_s_sets_feedback_when_no_columns_selected() {
        let mut app = App::new();
        app.selected_columns = [false; 10];
        handle_column_options_key(&mut app, KeyCode::Char('s'));
        assert!(app.options_feedback.is_some());
        assert!(!matches!(app.state, AppState::InProgress));
    }

    #[test]
    fn column_options_enter_on_last_row_starts_game() {
        let mut app = App::new();
        app.options_selection = COLUMN_LABELS.len();
        handle_column_options_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.state, AppState::InProgress));
    }

    #[test]
    fn column_options_down_clamps_at_last_row() {
        let mut app = App::new();
        app.options_selection = COLUMN_LABELS.len();
        handle_column_options_key(&mut app, KeyCode::Down);
        assert_eq!(app.options_selection, COLUMN_LABELS.len());
    }

    #[test]
    fn column_options_up_clamps_at_zero() {
        let mut app = App::new();
        app.options_selection = 0;
        handle_column_options_key(&mut app, KeyCode::Up);
        assert_eq!(app.options_selection, 0);
    }

    #[test]
    fn finished_esc_stops_running() {
        let mut app = App::new();
        handle_finished_key(&mut app, KeyCode::Esc);
        assert!(!app.running);
    }

    #[test]
    fn finished_enter_returns_to_menu() {
        let mut app = App::new();
        app.state = AppState::Finished;
        handle_finished_key(&mut app, KeyCode::Enter);
        assert!(app.running);
        assert!(matches!(app.state, AppState::Menu));
    }

    #[test]
    fn column_unlocked_esc_returns_to_menu() {
        let mut app = App::new();
        handle_column_unlocked_key(&mut app, KeyCode::Esc);
        assert!(matches!(app.state, AppState::Menu));
    }

    #[test]
    fn column_unlocked_enter_advances_to_in_progress() {
        let mut app = App::new();
        app.newly_unlocked_column = Some(1);
        app.selected_columns = [false; 10];
        app.selected_columns[0] = true;
        app.refill_deck();
        handle_column_unlocked_key(&mut app, KeyCode::Enter);
        assert!(matches!(app.state, AppState::InProgress));
        assert!(app.newly_unlocked_column.is_none());
        assert!(app.last_feedback.is_none());
        assert!(app.last_correct.is_none());
    }
}
