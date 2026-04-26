mod app;
mod input;
mod kana;
mod scoreboard;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::Result;
use app::{App, AppState};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use input::{
    handle_column_options_key, handle_column_unlocked_key, handle_finished_key,
    handle_in_progress_key, handle_menu_key, handle_quit_prompt_key, handle_showing_feedback_key,
};
use ratatui::Terminal;
use scoreboard::ScoreBoard;

fn main() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    app_result
}

fn run_app(terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stdout>>) -> Result<()> {
    let mut app = App::new();
    app.scoreboard = ScoreBoard::load();

    while app.running {
        app.update_session_timer();
        terminal.draw(|frame| ui::ui(frame, &app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        if app.quit_prompt.is_some() {
            handle_quit_prompt_key(&mut app, key.code);
        } else {
            match app.state {
                AppState::Menu => handle_menu_key(&mut app, key.code),
                AppState::ColumnOptions => handle_column_options_key(&mut app, key.code),
                AppState::InProgress => handle_in_progress_key(&mut app, key.code),
                AppState::ShowingFeedback => handle_showing_feedback_key(&mut app, key.code),
                AppState::ColumnUnlocked => handle_column_unlocked_key(&mut app, key.code),
                AppState::Finished => handle_finished_key(&mut app, key.code),
            }
        }
    }

    Ok(())
}
