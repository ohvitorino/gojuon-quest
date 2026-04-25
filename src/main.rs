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

const COLUMN_LABELS: [&str; 10] = [
    "Vowels",
    "K",
    "S",
    "T",
    "N",
    "H",
    "M",
    "Y",
    "R",
    "W",
];

const COLUMN_INDEX_GROUPS: [&[usize]; 10] = [
    &[0, 1, 2, 3, 4],
    &[5, 6, 7, 8, 9],
    &[10, 11, 12, 13, 14],
    &[15, 16, 17, 18, 19],
    &[20, 21, 22, 23, 24],
    &[25, 26, 27, 28, 29],
    &[30, 31, 32, 33, 34],
    &[35, 36, 37],
    &[38, 39, 40, 41, 42],
    &[43, 44, 45],
];

enum GameMode {
    Infinite,
    BestOf(u32),
}

enum RenderStyle {
    Braille,
    Ascii,
}

enum AppState {
    Menu,
    ColumnOptions,
    InProgress,
    ShowingFeedback,
    Finished,
}

struct App {
    running: bool,
    state: AppState,
    mode: GameMode,
    render_style: RenderStyle,
    menu_selection: usize,
    selected_columns: [bool; 10],
    options_selection: usize,
    options_feedback: Option<String>,
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
        }
    }

    fn refill_deck(&mut self) {
        self.deck = self.allowed_indices();
        self.deck.shuffle(&mut rand::rng());
        self.deck_position = 0;
    }

    fn allowed_indices(&self) -> Vec<usize> {
        let mut indices = Vec::new();
        for (column, enabled) in self.selected_columns.iter().enumerate() {
            if !enabled {
                continue;
            }
            indices.extend_from_slice(COLUMN_INDEX_GROUPS[column]);
        }
        indices
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

    fn render_style_label(&self) -> &'static str {
        match self.render_style {
            RenderStyle::Braille => "Braille",
            RenderStyle::Ascii => "Ascii",
        }
    }

    fn toggle_render_style(&mut self) {
        self.render_style = match self.render_style {
            RenderStyle::Braille => RenderStyle::Ascii,
            RenderStyle::Ascii => RenderStyle::Braille,
        };
    }

    fn prepare_selected_mode(&mut self) {
        self.mode = self.select_mode();
        self.options_selection = 0;
        self.options_feedback = None;
        self.state = AppState::ColumnOptions;
    }

    fn start_selected_mode(&mut self) {
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
            AppState::ColumnOptions => handle_column_options_key(&mut app, key.code),
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
        KeyCode::Down | KeyCode::Char('j') => app.menu_selection = (app.menu_selection + 1).min(2),
        KeyCode::Left | KeyCode::Right => {
            if app.menu_selection == 2 {
                app.toggle_render_style();
            }
        }
        KeyCode::Enter => {
            if app.menu_selection == 2 {
                app.toggle_render_style();
                return;
            }
            app.prepare_selected_mode();
        }
        _ => {}
    }
}

fn handle_column_options_key(app: &mut App, code: KeyCode) {
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

fn handle_finished_key(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Enter => app.running = false,
        _ => {}
    }
}

fn ui(frame: &mut Frame, app: &mut App) {
    match app.state {
        AppState::Menu => render_menu(frame, app),
        AppState::ColumnOptions => render_column_options(frame, app),
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
    let render_style_style = if app.menu_selection == 2 {
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
        Line::from(vec![
            Span::raw(if app.menu_selection == 2 { "> " } else { "  " }),
            Span::styled(
                format!("Render: {}", app.render_style_label()),
                render_style_style,
            ),
        ]),
        Line::from(""),
        Line::from("Use Up/Down to choose"),
        Line::from("Enter: select  |  Esc: quit"),
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

fn render_column_options(frame: &mut Frame, app: &App) {
    let mut lines = vec![
        Line::from("Select columns for this session"),
        Line::from(""),
    ];

    for (idx, label) in COLUMN_LABELS.iter().enumerate() {
        let marker = if app.options_selection == idx { "> " } else { "  " };
        let checkbox = if app.selected_columns[idx] { "[x]" } else { "[ ]" };
        let style = if app.options_selection == idx {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::raw(marker),
            Span::styled(format!("{} {}", checkbox, label), style),
        ]));
    }

    lines.push(Line::from(""));
    let start_selected = app.options_selection == COLUMN_LABELS.len();
    let start_style = if start_selected {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    lines.push(Line::from(vec![
        Span::raw(if start_selected { "> " } else { "  " }),
        Span::styled("Start", start_style),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(
        app.options_feedback
            .as_deref()
            .unwrap_or("Space/Enter: toggle  |  s/Enter on Start: begin  |  Esc: back"),
    ));

    let block = Paragraph::new(lines)
        .alignment(Alignment::Left)
        .block(Block::default().title("Columns").borders(Borders::ALL));

    let row = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(frame.area());

    let col = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(15),
            Constraint::Percentage(70),
            Constraint::Percentage(15),
        ])
        .split(row[1]);

    frame.render_widget(block, col[1]);
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

    match app.render_style {
        RenderStyle::Braille => render_hiragana_pixel_art(frame, app.current_hiragana(), glyph_col[1]),
        RenderStyle::Ascii => render_hiragana_ascii_art(frame, app.current_hiragana(), glyph_col[1]),
    }

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

fn render_hiragana_ascii_art(frame: &mut Frame, hiragana: &str, area: ratatui::layout::Rect) {
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

    let (metrics, bitmap) = font.rasterize(ch, 192.0f32);
    if metrics.width == 0 || metrics.height == 0 || area.width == 0 || area.height == 0 {
        return;
    }

    let target_w = area.width as usize;
    let target_h = area.height as usize;
    let src_w = metrics.width;
    let src_h = metrics.height;

    let mut min_x = src_w;
    let mut min_y = src_h;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut has_ink = false;
    for y in 0..src_h {
        for x in 0..src_w {
            let v = bitmap[y * src_w + x];
            if v <= 2 {
                continue;
            }
            has_ink = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !has_ink {
        return;
    }

    let crop_w = (max_x - min_x + 1) as f32;
    let crop_h = (max_y - min_y + 1) as f32;
    let target_wf = target_w as f32;
    let target_hf = target_h as f32;
    let scale = (target_wf / crop_w).min(target_hf / crop_h);
    let render_w = (crop_w * scale).max(1.0).round() as usize;
    let render_h = (crop_h * scale).max(1.0).round() as usize;
    let pad_x = (target_w.saturating_sub(render_w)) / 2;
    let pad_y = (target_h.saturating_sub(render_h)) / 2;

    // Supersample each terminal cell as a 2x2 block from the source raster.
    let super_w = render_w * 2;
    let super_h = render_h * 2;
    let mut supersampled = vec![0u8; super_w * super_h];
    for y in 0..super_h {
        let src_yf = min_y as f32 + ((y as f32 + 0.5) / super_h as f32) * crop_h;
        let src_y = src_yf.floor().clamp(0.0, (src_h - 1) as f32) as usize;
        for x in 0..super_w {
            let src_xf = min_x as f32 + ((x as f32 + 0.5) / super_w as f32) * crop_w;
            let src_x = src_xf.floor().clamp(0.0, (src_w - 1) as f32) as usize;
            supersampled[y * super_w + x] = bitmap[src_y * src_w + src_x];
        }
    }

    let mut rows = Vec::with_capacity(target_h);
    let shades = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

    for y in 0..target_h {
        let mut line = String::with_capacity(target_w);
        for x in 0..target_w {
            let in_render_x = x >= pad_x && x < pad_x + render_w;
            let in_render_y = y >= pad_y && y < pad_y + render_h;
            if !in_render_x || !in_render_y {
                line.push(' ');
                continue;
            }

            let sx = (x - pad_x) * 2;
            let sy = (y - pad_y) * 2;
            let a = supersampled[sy * super_w + sx] as u32;
            let b = supersampled[sy * super_w + sx + 1] as u32;
            let c = supersampled[(sy + 1) * super_w + sx] as u32;
            let d = supersampled[(sy + 1) * super_w + sx + 1] as u32;
            let avg = (a + b + c + d) / 4;

            let shade_index = (avg * (shades.len() as u32 - 1) / 255) as usize;
            line.push(shades[shade_index]);
        }
        rows.push(line);
    }

    let art = Paragraph::new(rows.join("\n"))
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::White));
    frame.render_widget(art, area);
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
