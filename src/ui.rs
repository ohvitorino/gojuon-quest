use std::sync::OnceLock;

use fontdue::Font;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::canvas::{Canvas, Points};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, AppState, GameMode, RenderStyle};
use crate::kana::{COLUMN_INDEX_GROUPS, COLUMN_LABELS, HIRAGANA_BASIC_46};

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
        let fallback = Paragraph::new(hiragana).alignment(Alignment::Center).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
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

fn render_hiragana_ascii_art(frame: &mut Frame, hiragana: &str, area: ratatui::layout::Rect) {
    let Some(font) = pixel_font() else {
        let fallback = Paragraph::new(hiragana).alignment(Alignment::Center).style(
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );
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

pub(crate) fn ui(frame: &mut Frame, app: &mut App) {
    match app.state {
        AppState::Menu => render_menu(frame, app),
        AppState::ColumnOptions => render_column_options(frame, app),
        AppState::InProgress | AppState::ShowingFeedback => {
            if matches!(app.mode, GameMode::Progressive) {
                render_progressive_game_screen(frame, app);
            } else {
                render_game_screen(frame, app);
            }
        }
        AppState::ColumnUnlocked => render_column_unlocked(frame, app),
        AppState::Finished => render_finished(frame, app),
    }
}

fn render_menu(frame: &mut Frame, app: &App) {
    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),
            Constraint::Min(18),
            Constraint::Length(4),
        ])
        .split(frame.area());
    let header = Paragraph::new(vec![
        Line::from(Span::styled(
            "GOJUON QUEST",
            Style::default()
                .fg(Color::LightYellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            "祭り Selection Hall",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("Choose your training path"),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title("Main Gate"));
    frame.render_widget(header, shell[0]);

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(65), Constraint::Percentage(35)])
        .split(shell[1]);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(6),
        ])
        .split(body[0]);

    let mode_cards = [
        ("Infinite", "Endless reps, free pace"),
        ("Best of 20", "Focused sprint session"),
        ("Progressive", "Unlock columns by mastery"),
    ];
    for (idx, area) in left.iter().take(3).enumerate() {
        let selected = app.menu_selection == idx;
        let accent = if selected {
            Color::LightYellow
        } else {
            Color::DarkGray
        };
        let lines = vec![
            Line::from(vec![
                Span::styled(
                    if selected { "❯ " } else { "  " },
                    Style::default().fg(accent),
                ),
                Span::styled(
                    mode_cards[idx].0,
                    Style::default().fg(accent).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                mode_cards[idx].1,
                Style::default().fg(Color::Gray),
            )),
        ];
        let card = Paragraph::new(lines).block(Block::default().borders(Borders::ALL));
        frame.render_widget(card, *area);
    }

    let render_selected = app.menu_selection == 3;
    let render_color = if render_selected {
        Color::LightMagenta
    } else {
        Color::DarkGray
    };
    let render_card = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                if render_selected { "❯ " } else { "  " },
                Style::default().fg(render_color),
            ),
            Span::styled(
                "Render Style",
                Style::default()
                    .fg(render_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Current: ", Style::default().fg(Color::Gray)),
            Span::styled(
                app.render_style_label(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(Span::styled(
            "Use ←/→ or h/l to switch",
            Style::default().fg(Color::Gray),
        )),
    ])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .title("Style"),
    );
    frame.render_widget(render_card, left[3]);

    let side_panel = Paragraph::new(vec![
        Line::from(Span::styled(
            "Dojo Notes",
            Style::default()
                .fg(Color::LightCyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Infinite"),
        Line::from("  open-ended practice"),
        Line::from(""),
        Line::from("Best of 20"),
        Line::from("  quick score challenge"),
        Line::from(""),
        Line::from("Progressive"),
        Line::from("  master each row to unlock"),
    ])
    .alignment(Alignment::Left)
    .block(Block::default().borders(Borders::ALL).title("Info"));
    frame.render_widget(side_panel, body[1]);

    let footer = Paragraph::new(vec![
        Line::from(Span::styled(
            "Primary: ↑/↓ or j/k to move  •  Enter to select",
            Style::default().fg(Color::White),
        )),
        Line::from(Span::styled(
            "Secondary: ←/→ or h/l on Render row  •  Esc to quit",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL).title("Controls"));
    frame.render_widget(footer, shell[2]);
}

fn render_column_options(frame: &mut Frame, app: &App) {
    let shell = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(16),
            Constraint::Length(5),
        ])
        .split(frame.area());
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Column Selection Shrine",
                Style::default()
                    .fg(Color::LightYellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("Choose your active kana rows"),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Session Setup"),
        ),
        shell[0],
    );

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(shell[1]);

    let mut rows_constraints = vec![Constraint::Length(2); COLUMN_LABELS.len()];
    rows_constraints.push(Constraint::Length(1));
    rows_constraints.push(Constraint::Length(3));
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(rows_constraints)
        .split(body[0]);

    for (idx, label) in COLUMN_LABELS.iter().enumerate() {
        let focused = app.options_selection == idx;
        let active = app.selected_columns[idx];
        let accent = if focused {
            Color::LightYellow
        } else if active {
            Color::LightGreen
        } else {
            Color::Gray
        };
        let state_badge = if active { "ON " } else { "OFF" };
        let focus_marker = if focused { "❯" } else { " " };
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{focus_marker} "), Style::default().fg(accent)),
                Span::styled(format!("[{state_badge}] "), Style::default().fg(accent)),
                Span::styled(
                    *label,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]))
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT)),
            rows[idx],
        );
    }

    let start_selected = app.options_selection == COLUMN_LABELS.len();
    let start_color = if start_selected {
        Color::LightMagenta
    } else {
        Color::DarkGray
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    if start_selected { "❯ " } else { "  " },
                    Style::default().fg(start_color),
                ),
                Span::styled(
                    "START SESSION",
                    Style::default()
                        .fg(start_color)
                        .add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(Span::styled(
                "Press s or Enter",
                Style::default().fg(Color::Gray),
            )),
        ])
        .block(Block::default().borders(Borders::ALL).title("Action")),
        rows[COLUMN_LABELS.len() + 1],
    );

    let active_count = app
        .selected_columns
        .iter()
        .filter(|enabled| **enabled)
        .count();
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(
                "Selection",
                Style::default()
                    .fg(Color::LightCyan)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "Active columns: {active_count}/{}",
                COLUMN_LABELS.len()
            )),
            Line::from(if active_count == 0 {
                "Need at least one column"
            } else {
                "Ready to begin"
            }),
        ])
        .block(Block::default().borders(Borders::ALL).title("Status")),
        body[1],
    );

    let feedback = app
        .options_feedback
        .as_deref()
        .unwrap_or("Primary: ↑/↓ move  •  Enter/Space toggle  •  s begin");
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(feedback, Style::default().fg(Color::White))),
            Line::from(Span::styled(
                "Secondary: Esc back",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Hints")),
        shell[2],
    );
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
        GameMode::Progressive => format!("{}/{}", app.correct, app.incorrect),
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
        RenderStyle::Braille => {
            render_hiragana_pixel_art(frame, app.current_hiragana(), glyph_col[1])
        }
        RenderStyle::Ascii => {
            render_hiragana_ascii_art(frame, app.current_hiragana(), glyph_col[1])
        }
    }

    let answer = Paragraph::new(app.input.as_str()).alignment(Alignment::Center);
    frame.render_widget(answer, layout[3]);

    let feedback_color = match app.last_correct {
        Some(true) => Color::Green,
        Some(false) => Color::Red,
        None => Color::Reset,
    };
    let feedback_style = if showing_feedback {
        Style::default()
            .fg(feedback_color)
            .add_modifier(Modifier::BOLD)
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

fn render_progressive_game_screen(frame: &mut Frame, app: &mut App) {
    let showing_feedback = matches!(app.state, AppState::ShowingFeedback);
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(6),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let progress_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(vec![
            Constraint::Ratio(1, COLUMN_LABELS.len() as u32);
            COLUMN_LABELS.len()
        ])
        .split(layout[0]);
    for (column, area) in progress_chunks.iter().enumerate() {
        let total = (COLUMN_INDEX_GROUPS[column].len() * 3) as u32;
        let progress = app.column_progress(column);
        let text = if app.is_column_mastered(column) {
            format!("{} ✓", COLUMN_LABELS[column])
        } else if column == app.progressive_unlocked_columns.saturating_sub(1) {
            format!("{} {}/{}", COLUMN_LABELS[column], progress, total)
        } else if column < app.progressive_unlocked_columns {
            format!("{} ..", COLUMN_LABELS[column])
        } else {
            format!("{} ···", COLUMN_LABELS[column])
        };
        let style = if column == app.progressive_unlocked_columns.saturating_sub(1) {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        frame.render_widget(Paragraph::new(text).style(style), *area);
    }

    let active_column = app.progressive_unlocked_columns.saturating_sub(1);
    let mastery_line = COLUMN_INDEX_GROUPS[active_column]
        .iter()
        .map(|index| {
            let (kana, _) = HIRAGANA_BASIC_46[*index];
            let dots = (0..3)
                .map(|dot| {
                    if dot < app.kana_correct_counts[*index].min(3) as usize {
                        '●'
                    } else {
                        '○'
                    }
                })
                .collect::<String>();
            format!("{kana}{dots}")
        })
        .collect::<Vec<_>>()
        .join(" ");
    frame.render_widget(
        Paragraph::new(mastery_line).alignment(Alignment::Center),
        layout[1],
    );

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
        RenderStyle::Braille => {
            render_hiragana_pixel_art(frame, app.current_hiragana(), glyph_col[1])
        }
        RenderStyle::Ascii => {
            render_hiragana_ascii_art(frame, app.current_hiragana(), glyph_col[1])
        }
    }

    frame.render_widget(
        Paragraph::new(app.input.as_str()).alignment(Alignment::Center),
        layout[3],
    );

    let feedback_color = match app.last_correct {
        Some(true) => Color::Green,
        Some(false) => Color::Red,
        None => Color::Reset,
    };
    let feedback_style = if showing_feedback {
        Style::default()
            .fg(feedback_color)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().add_modifier(Modifier::DIM)
    };
    let feedback_text = app
        .last_feedback
        .as_deref()
        .unwrap_or("Type romaji and press Enter");
    frame.render_widget(
        Paragraph::new(feedback_text)
            .alignment(Alignment::Center)
            .style(feedback_style),
        layout[4],
    );

    let total = app.correct + app.incorrect;
    let status_text = if showing_feedback {
        format!(
            "Streak: {}  Max: {}  |  Accuracy: {:.1}%  |  Total: {}  |  Enter/Space: next  Esc: finish session",
            app.streak,
            app.max_streak,
            app.accuracy(),
            total
        )
    } else {
        format!(
            "Streak: {}  Max: {}  |  Accuracy: {:.1}%  |  Total: {}  |  Enter: evaluate  Backspace: delete  Esc: finish session",
            app.streak,
            app.max_streak,
            app.accuracy(),
            total
        )
    };
    frame.render_widget(
        Paragraph::new(status_text)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM)),
        layout[5],
    );
}

fn render_finished(frame: &mut Frame, app: &App) {
    if matches!(app.mode, GameMode::Progressive) {
        let total = app.correct + app.incorrect;
        let hardest = app
            .hardest_column()
            .map(|column| {
                let attempts = app.column_attempts[column];
                let correct = app.column_correct[column];
                let accuracy = if attempts == 0 {
                    0.0
                } else {
                    (correct as f64 / attempts as f64) * 100.0
                };
                format!("{} ({:.1}%)", COLUMN_LABELS[column], accuracy)
            })
            .unwrap_or_else(|| "N/A".to_string());
        let avg_questions = if COLUMN_LABELS.is_empty() {
            0.0
        } else {
            total as f64 / COLUMN_LABELS.len() as f64
        };
        let summary = format!(
            "Session Finished\n\nMode: Progressive\nColumns mastered: {}/{}\nCorrect: {}\nIncorrect: {}\nTotal: {}\nAccuracy: {:.1}%\nBest streak: {}\nAvg questions per column: {:.1}\nHardest column: {}\n\nPress Esc or Enter to exit.",
            app.progressive_unlocked_columns.min(COLUMN_LABELS.len()),
            COLUMN_LABELS.len(),
            app.correct,
            app.incorrect,
            total,
            app.accuracy(),
            app.max_streak,
            avg_questions,
            hardest
        );

        let block = Paragraph::new(summary).alignment(Alignment::Center).block(
            Block::default()
                .title("Final Results")
                .borders(Borders::ALL),
        );
        frame.render_widget(block, frame.area());
        return;
    }

    let total = app.correct + app.incorrect;
    let summary = format!(
        "Session Finished\n\nCorrect: {}\nIncorrect: {}\nTotal: {}\nAccuracy: {:.1}%\n\nPress Esc or Enter to exit.",
        app.correct,
        app.incorrect,
        total,
        app.accuracy()
    );

    let block = Paragraph::new(summary).alignment(Alignment::Center).block(
        Block::default()
            .title("Final Results")
            .borders(Borders::ALL),
    );
    frame.render_widget(block, frame.area());
}

fn render_column_unlocked(frame: &mut Frame, app: &App) {
    let Some(column) = app.newly_unlocked_column else {
        return;
    };
    let kana = COLUMN_INDEX_GROUPS[column]
        .iter()
        .map(|index| HIRAGANA_BASIC_46[*index].0)
        .collect::<Vec<_>>()
        .join(" ");
    let questions = app.correct + app.incorrect;
    let mastered = column;
    let body = format!(
        "Column Unlocked!\n\n{}-row ({})\nadded to your deck\n\nColumns mastered: {} / {}\nQuestions so far: {}\nSession accuracy: {:.1}%\n\nPress Enter to continue",
        COLUMN_LABELS[column],
        kana,
        mastered,
        COLUMN_LABELS.len(),
        questions,
        app.accuracy()
    );

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
            Constraint::Percentage(25),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
        ])
        .split(row[1]);
    frame.render_widget(
        Paragraph::new(body)
            .alignment(Alignment::Center)
            .block(Block::default().title("Progressive").borders(Borders::ALL)),
        col[1],
    );
}
