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
        KeyCode::Up | KeyCode::Char('k') => app.menu_selection = app.menu_selection.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => app.menu_selection = (app.menu_selection + 1).min(3),
        KeyCode::Left | KeyCode::Right => {
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
            app.selected_columns[app.options_selection] = !app.selected_columns[app.options_selection];
            app.options_feedback = None;
        }
        _ => {}
    }
}

pub(crate) fn handle_finished_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Enter => app.running = false,
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
