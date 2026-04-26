use crossterm::event::KeyCode;
use gojuon_core::actions::CoreAction;

use crate::app::{App, AppState};
use crate::kana::COLUMN_LABELS;

pub(crate) fn handle_quit_prompt_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.dispatch(CoreAction::ConfirmPrompt)
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.dispatch(CoreAction::CancelPrompt)
        }
        _ => {}
    }
}

pub(crate) fn handle_in_progress_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.dispatch(CoreAction::OpenAbandonPrompt),
        KeyCode::Enter => app.dispatch(CoreAction::SubmitAnswer),
        KeyCode::Backspace => app.dispatch(CoreAction::Backspace),
        KeyCode::Char(c) => app.dispatch(CoreAction::InputChar(c)),
        _ => {}
    }
}

pub(crate) fn handle_showing_feedback_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.dispatch(CoreAction::OpenAbandonPrompt),
        KeyCode::Enter | KeyCode::Char(' ') => app.dispatch(CoreAction::ContinueAfterFeedback),
        _ => {}
    }
}

pub(crate) fn handle_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.dispatch(CoreAction::OpenExitPrompt),
        KeyCode::Up | KeyCode::Char('k') => app.dispatch(CoreAction::MenuUp),
        KeyCode::Down | KeyCode::Char('j') => app.dispatch(CoreAction::MenuDown),
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
            app.dispatch(CoreAction::StartFromMenu);
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
        KeyCode::Up | KeyCode::Char('k') => app.dispatch(CoreAction::OptionsUp),
        KeyCode::Down | KeyCode::Char('j') => app.dispatch(CoreAction::OptionsDown),
        KeyCode::Char('s') => app.dispatch(CoreAction::StartFromOptions),
        KeyCode::Enter | KeyCode::Char(' ') => {
            if app.options_selection <= last_row {
                app.dispatch(CoreAction::ToggleOptionOrStart);
            }
        }
        _ => {}
    }
}

pub(crate) fn handle_finished_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.dispatch(CoreAction::OpenExitPrompt),
        KeyCode::Enter => app.dispatch(CoreAction::FinishedToMenu),
        _ => {}
    }
}

pub(crate) fn handle_column_unlocked_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.dispatch(CoreAction::OpenAbandonPrompt),
        KeyCode::Enter | KeyCode::Char(' ') => app.dispatch(CoreAction::ContinueAfterUnlock),
        _ => {}
    }
}
