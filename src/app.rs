use std::ops::{Deref, DerefMut};
use std::time::Instant;

use gojuon_core::actions::{CoreAction, CoreEffect};
use gojuon_core::reducer::reduce;
use gojuon_core::state::GameState;
pub(crate) use gojuon_core::state::{GameMode, GameStateKind as AppState, QuitPrompt};

use crate::scoreboard::{ScoreBoard, ScoreEntry};

pub(crate) enum RenderStyle {
    Braille,
    Ascii,
}

pub(crate) struct App {
    pub(crate) game: GameState,
    pub(crate) render_style: RenderStyle,
    pub(crate) session_started_at: Option<Instant>,
    pub(crate) scoreboard: ScoreBoard,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            game: GameState::default(),
            render_style: RenderStyle::Ascii,
            session_started_at: None,
            scoreboard: ScoreBoard::default(),
        }
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
        self.dispatch(CoreAction::SetElapsedSeconds(
            started_at.elapsed().as_secs(),
        ));
    }

    pub(crate) fn dispatch(&mut self, action: CoreAction) {
        if matches!(action, CoreAction::StartFromMenu) && self.menu_selection == 1 {
            self.session_started_at = Some(Instant::now());
        }
        for effect in reduce(&mut self.game, action) {
            self.handle_effect(effect);
        }
    }

    fn handle_effect(&mut self, effect: CoreEffect) {
        match effect {
            CoreEffect::PersistBestOfScore {
                correct,
                incorrect,
                elapsed_secs,
                points,
            } => {
                self.scoreboard.add_entry(ScoreEntry::now(
                    correct,
                    incorrect,
                    elapsed_secs,
                    points,
                ));
                let _ = self.scoreboard.save();
            }
        }
    }
}

impl Deref for App {
    type Target = GameState;

    fn deref(&self) -> &Self::Target {
        &self.game
    }
}

impl DerefMut for App {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.game
    }
}
