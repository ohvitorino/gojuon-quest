use crate::actions::{CoreAction, CoreEffect};
use crate::kana::COLUMN_LABELS;
use crate::state::{GameMode, GameState, GameStateKind, QuitPrompt};

pub fn reduce(state: &mut GameState, action: CoreAction) -> Vec<CoreEffect> {
    let mut effects = Vec::new();
    match action {
        CoreAction::MenuUp => state.menu_selection = state.menu_selection.saturating_sub(1),
        CoreAction::MenuDown => state.menu_selection = (state.menu_selection + 1).min(3),
        CoreAction::StartFromMenu => state.prepare_selected_mode(),
        CoreAction::OpenExitPrompt => state.quit_prompt = Some(QuitPrompt::ExitApplication),
        CoreAction::OpenAbandonPrompt => state.quit_prompt = Some(QuitPrompt::AbandonSession),
        CoreAction::ConfirmPrompt => {
            if let Some(prompt) = state.quit_prompt {
                match prompt {
                    QuitPrompt::ExitApplication => state.running = false,
                    QuitPrompt::AbandonSession => state.state = GameStateKind::Menu,
                }
                state.quit_prompt = None;
            }
        }
        CoreAction::CancelPrompt => state.quit_prompt = None,
        CoreAction::OptionsUp => {
            state.options_selection = state.options_selection.saturating_sub(1)
        }
        CoreAction::OptionsDown => {
            state.options_selection = (state.options_selection + 1).min(COLUMN_LABELS.len())
        }
        CoreAction::ToggleOptionOrStart => {
            if state.options_selection == COLUMN_LABELS.len() {
                if state.allowed_indices().is_empty() {
                    state.options_feedback =
                        Some("Enable at least one column to start".to_string());
                } else {
                    state.options_feedback = None;
                    state.start_selected_mode();
                }
            } else {
                let idx = state.options_selection;
                state.selected_columns[idx] = !state.selected_columns[idx];
                state.options_feedback = None;
            }
        }
        CoreAction::StartFromOptions => {
            if state.allowed_indices().is_empty() {
                state.options_feedback = Some("Enable at least one column to start".to_string());
            } else {
                state.options_feedback = None;
                state.start_selected_mode();
            }
        }
        CoreAction::InputChar(c) => state.input.push(c),
        CoreAction::Backspace => {
            state.input.pop();
        }
        CoreAction::SubmitAnswer => {
            state.evaluate_current_answer();
            if matches!(state.mode, GameMode::BestOf(_))
                && matches!(state.state, GameStateKind::Finished)
                && !state.recorded_score_for_session
            {
                effects.push(CoreEffect::PersistBestOfScore {
                    correct: state.correct,
                    incorrect: state.incorrect,
                    elapsed_secs: state.session_elapsed_secs,
                    points: state.best_of_points(),
                });
                state.recorded_score_for_session = true;
            }
        }
        CoreAction::ContinueAfterFeedback => {
            state.advance_prompt();
            state.last_feedback = None;
            state.last_correct = None;
            state.state = GameStateKind::InProgress;
        }
        CoreAction::ContinueAfterUnlock => {
            state.newly_unlocked_column = None;
            state.advance_prompt();
            state.last_feedback = None;
            state.last_correct = None;
            state.state = GameStateKind::InProgress;
        }
        CoreAction::FinishedToMenu => state.state = GameStateKind::Menu,
        CoreAction::SetElapsedSeconds(seconds) => {
            state.session_elapsed_secs = seconds;
        }
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_answer_emits_best_of_persist_once() {
        let mut state = GameState {
            mode: GameMode::BestOf(1),
            current_index: 0,
            input: "a".to_string(),
            session_elapsed_secs: 12,
            ..GameState::default()
        };

        let first = reduce(&mut state, CoreAction::SubmitAnswer);
        assert_eq!(first.len(), 1);
        assert!(state.recorded_score_for_session);

        state.current_index = 0;
        state.input = "a".to_string();
        let second = reduce(&mut state, CoreAction::SubmitAnswer);
        assert!(second.is_empty());
    }
}
