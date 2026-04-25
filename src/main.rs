use std::io;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use fontdue::Font;
use rand::seq::SliceRandom;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

static PIXEL_FONT: OnceLock<Option<Font>> = OnceLock::new();

fn pixel_font() -> Option<&'static Font> {
    PIXEL_FONT
        .get_or_init(|| {
            let candidates = [
                "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
                "/System/Library/Fonts/Hiragino Sans GB.ttc",
                "/System/Library/Fonts/AquaKana.ttc",
            ];
            candidates.iter().find_map(|path| {
                let data = std::fs::read(path).ok()?;
                Font::from_bytes(data.as_slice(), fontdue::FontSettings::default()).ok()
            })
        })
        .as_ref()
}

fn render_hiragana_pixel_art(frame: &mut Frame, hiragana: &str, area: ratatui::layout::Rect) {
    let Some(font) = pixel_font() else {
        let fallback = Paragraph::new(hiragana)
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::White).add_modifier(Modifier::BOLD));
        frame.render_widget(fallback, area);
        return;
    };

    let Some(ch) = hiragana.chars().next() else {
        return;
    };

    let px = 64.0f32;
    let (metrics, bitmap) = font.rasterize(ch, px);

    if metrics.width == 0 || metrics.height == 0 {
        return;
    }

    let w = metrics.width as f64;
    let h = metrics.height as f64;

    let points: Vec<(f64, f64)> = bitmap
        .iter()
        .enumerate()
        .filter(|&(_, &v)| v > 4)
        .map(|(i, _)| {
            let col = (i % metrics.width) as f64 + 0.5;
            let row = (i / metrics.width) as f64;
            (col, h - row - 0.5)
        })
        .collect();

    let canvas = Canvas::default()
        .marker(Marker::Braille)
        .x_bounds([0.0, w])
        .y_bounds([0.0, h])
        .paint(move |ctx| {
            ctx.draw(&Points {
                coords: &points,
                color: Color::White,
            });
        });

    frame.render_widget(canvas, area);
}

const HIRAGANA_BASIC_46: [(&str, &str); 46] = [
    ("あ", "a"),
    ("い", "i"),
    ("う", "u"),
    ("え", "e"),
    ("お", "o"),
    ("か", "ka"),
    ("き", "ki"),
    ("く", "ku"),
    ("け", "ke"),
    ("こ", "ko"),
    ("さ", "sa"),
    ("し", "shi"),
    ("す", "su"),
    ("せ", "se"),
    ("そ", "so"),
    ("た", "ta"),
    ("ち", "chi"),
    ("つ", "tsu"),
    ("て", "te"),
    ("と", "to"),
    ("な", "na"),
    ("に", "ni"),
    ("ぬ", "nu"),
    ("ね", "ne"),
    ("の", "no"),
    ("は", "ha"),
    ("ひ", "hi"),
    ("ふ", "fu"),
    ("へ", "he"),
    ("ほ", "ho"),
    ("ま", "ma"),
    ("み", "mi"),
    ("む", "mu"),
    ("め", "me"),
    ("も", "mo"),
    ("や", "ya"),
    ("ゆ", "yu"),
    ("よ", "yo"),
    ("ら", "ra"),
    ("り", "ri"),
    ("る", "ru"),
    ("れ", "re"),
    ("ろ", "ro"),
    ("わ", "wa"),
    ("を", "wo"),
    ("ん", "n"),
];

enum GameMode {
    Infinite,
    BestOf(u32),
}

enum AppState {
    Menu,
    InProgress,
    ShowingFeedback,
    Finished,
}

struct App {
    running: bool,
    state: AppState,
    mode: GameMode,
    menu_selection: usize,
    input: String,
    correct: u32,
    incorrect: u32,
    last_feedback: Option<String>,
    last_correct: Option<bool>,
    deck: Vec<usize>,
    deck_position: usize,
    current_index: usize,
}

impl App {
    fn new() -> Self {
        Self {
            running: true,
            state: AppState::Menu,
            mode: GameMode::Infinite,
            menu_selection: 0,
            input: String::new(),
            correct: 0,
            incorrect: 0,
            last_feedback: None,
            last_correct: None,
            deck: Vec::new(),
            deck_position: 0,
            current_index: 0,
        }
    }

    fn refill_deck(&mut self) {
        self.deck = (0..HIRAGANA_BASIC_46.len()).collect();
        self.deck.shuffle(&mut rand::rng());
        self.deck_position = 0;
    }

    fn expected_romaji(&self) -> &str {
        HIRAGANA_BASIC_46[self.current_index].1
    }

    fn current_hiragana(&self) -> &str {
        HIRAGANA_BASIC_46[self.current_index].0
    }

    fn advance_prompt(&mut self) {
        if self.deck_position >= self.deck.len() {
            self.refill_deck();
        }

        self.current_index = self.deck[self.deck_position];
        self.deck_position += 1;
    }

    fn evaluate_current_answer(&mut self) {
        let expected = self.expected_romaji().to_string();
        let shown = self.current_hiragana().to_string();
        let typed = self.input.trim().to_ascii_lowercase();
        let is_correct = typed == expected;

        if is_correct {
            self.correct += 1;
            self.last_feedback = Some(format!("Correct: {} → {}", shown, expected));
        } else {
            self.incorrect += 1;
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

        self.state = AppState::ShowingFeedback;
    }

    fn reached_mode_limit(&self) -> bool {
        let answered = self.correct + self.incorrect;
        match self.mode {
            GameMode::Infinite => false,
            GameMode::BestOf(limit) => answered >= limit,
        }
    }

    fn select_mode(&self) -> GameMode {
        if self.menu_selection == 0 {
            return GameMode::Infinite;
        }

        GameMode::BestOf(20)
    }

    fn start_selected_mode(&mut self) {
        self.mode = self.select_mode();
        self.state = AppState::InProgress;
        self.input.clear();
        self.correct = 0;
        self.incorrect = 0;
        self.last_feedback = None;
        self.last_correct = None;
        self.refill_deck();
        self.current_index = self.deck[0];
        self.deck_position = 1;
    }

    fn accuracy(&self) -> f64 {
        let total = self.correct + self.incorrect;
        if total == 0 {
            return 0.0;
        }

        (self.correct as f64 / total as f64) * 100.0
    }
}

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

    while app.running {
        terminal.draw(|frame| ui(frame, &mut app))?;

        if !event::poll(Duration::from_millis(200))? {
            continue;
        }

        let Event::Key(key) = event::read()? else {
            continue;
        };

        if key.kind != KeyEventKind::Press {
            continue;
        }

        match app.state {
            AppState::Menu => handle_menu_key(&mut app, key.code),
            AppState::InProgress => handle_in_progress_key(&mut app, key.code),
            AppState::ShowingFeedback => handle_showing_feedback_key(&mut app, key.code),
            AppState::Finished => handle_finished_key(&mut app, key.code),
        }
    }

    Ok(())
}

fn handle_in_progress_key(app: &mut App, code: KeyCode) {
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

fn handle_showing_feedback_key(app: &mut App, code: KeyCode) {
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

fn handle_menu_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.running = false,
        KeyCode::Up | KeyCode::Char('k') => app.menu_selection = app.menu_selection.saturating_sub(1),
        KeyCode::Down | KeyCode::Char('j') => app.menu_selection = (app.menu_selection + 1).min(1),
        KeyCode::Enter => app.start_selected_mode(),
        _ => {}
    }
}

fn handle_finished_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Enter => app.running = false,
        _ => {}
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    match app.state {
        AppState::Menu => render_menu(frame, app),
        AppState::InProgress | AppState::ShowingFeedback => render_game_screen(frame, app),
        AppState::Finished => render_finished(frame, app),
    }
}

fn render_menu(frame: &mut Frame, app: &App) {
    let infinite_style = if app.menu_selection == 0 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let best_of_style = if app.menu_selection == 1 {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let menu_lines = vec![
        Line::from("Hiragana Quiz"),
        Line::from(""),
        Line::from(vec![
            Span::raw(if app.menu_selection == 0 { "> " } else { "  " }),
            Span::styled("Infinite", infinite_style),
        ]),
        Line::from(vec![
            Span::raw(if app.menu_selection == 1 { "> " } else { "  " }),
            Span::styled("Best of 20", best_of_style),
        ]),
        Line::from(""),
        Line::from("Use Up/Down to choose"),
        Line::from("Enter: start  |  Esc: quit"),
    ];

    let menu = Paragraph::new(menu_lines)
        .alignment(Alignment::Center)
        .block(Block::default().title("Game Mode").borders(Borders::ALL));

    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(frame.area());

    let col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(row[1]);

    frame.render_widget(menu, col[1]);
}

fn render_game_screen(frame: &mut Frame, app: &mut App) {
    let showing_feedback = matches!(app.state, AppState::ShowingFeedback);

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(8),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let top_bar = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(10)])
        .split(layout[0]);

    let score_text = match app.mode {
        GameMode::Infinite => format!("{}/{}", app.correct, app.incorrect),
        GameMode::BestOf(limit) => format!("{}/{}", app.correct + app.incorrect, limit),
    };
    let score = Paragraph::new(score_text).alignment(Alignment::Right);
    frame.render_widget(score, top_bar[1]);

    let glyph_row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(30),
            Constraint::Percentage(35),
        ])
        .split(layout[2]);

    let glyph_col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(glyph_row[1]);

    render_hiragana_pixel_art(frame, app.current_hiragana(), glyph_col[1]);

    let answer = Paragraph::new(app.input.as_str()).alignment(Alignment::Center);
    frame.render_widget(answer, layout[3]);

    let feedback_color = match app.last_correct {
        Some(true) => Color::Green,
        Some(false) => Color::Red,
        None => Color::Reset,
    };
    let feedback_style = if showing_feedback {
        Style::default().fg(feedback_color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let feedback_text = app
        .last_feedback
        .as_deref()
        .unwrap_or("Type romaji and press Enter");
    let feedback = Paragraph::new(feedback_text)
        .alignment(Alignment::Center)
        .style(feedback_style);
    frame.render_widget(feedback, layout[4]);

    let controls_text = if showing_feedback {
        "Enter/Space: next  |  Esc: finish session"
    } else {
        "Enter: evaluate  |  Backspace: delete  |  Esc: finish session"
    };
    let controls = Paragraph::new(controls_text)
        .alignment(Alignment::Center)
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(controls, layout[5]);
}

fn render_finished(frame: &mut Frame, app: &App) {
    let total = app.correct + app.incorrect;
    let summary = format!(
        "Session Finished\n\nCorrect: {}\nIncorrect: {}\nTotal: {}\nAccuracy: {:.1}%\n\nPress Esc or Enter to exit.",
        app.correct,
        app.incorrect,
        total,
        app.accuracy()
    );

    let block = Paragraph::new(summary)
        .alignment(Alignment::Center)
        .block(Block::default().title("Final Results").borders(Borders::ALL));
    frame.render_widget(block, frame.area());
}
