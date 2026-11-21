use crate::catalog::Candidate;
use crate::config::{Align, Popup, Position, Theme};
use crate::context::{Applied, Context};
use crate::engine::suggest_ranked;
use crate::usage::Usage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Margin, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io;

#[derive(Clone, Copy)]
struct Palette {
    border: Color,
    text: Color,
    muted: Color,
    background: Color,
    highlight: Color,
}

fn palette(theme: Theme) -> Palette {
    match theme {
        Theme::Violet => Palette {
            border: Color::Rgb(180, 135, 255),
            text: Color::White,
            muted: Color::Gray,
            background: Color::Rgb(30, 27, 43),
            highlight: Color::Rgb(89, 57, 143),
        },
        Theme::Graphite => Palette {
            border: Color::Gray,
            text: Color::White,
            muted: Color::DarkGray,
            background: Color::Rgb(32, 34, 38),
            highlight: Color::Rgb(67, 72, 82),
        },
        Theme::Light => Palette {
            border: Color::Rgb(85, 70, 148),
            text: Color::Black,
            muted: Color::DarkGray,
            background: Color::Rgb(240, 239, 246),
            highlight: Color::Rgb(197, 180, 237),
        },
        Theme::Amber => Palette {
            border: Color::Yellow,
            text: Color::White,
            muted: Color::Gray,
            background: Color::Rgb(38, 30, 20),
            highlight: Color::Rgb(112, 76, 24),
        },
    }
}

/// The card can be narrow even though Ratatui's inline viewport is terminal-wide.
/// Explicit above/below selects the edge *inside the reserved viewport*.
fn popup_rect(area: Rect, popup: &Popup) -> Rect {
    let width = popup.width.min(area.width);
    let height = popup.height.min(area.height);
    let x = match popup.align {
        Align::Left => area.x,
        Align::Center => area.x + (area.width - width) / 2,
        Align::Right => area.x + area.width - width,
    };
    let y = match popup.position {
        Position::Below => area.y + area.height - height,
        Position::Above | Position::Auto => area.y,
    };
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn viewport_height(popup: &Popup) -> u16 {
    match popup.position {
        Position::Auto => popup.height,
        Position::Above | Position::Below => popup.height.saturating_add(2),
    }
}

pub fn pick(
    catalog: &[Candidate],
    context: &Context,
    usage: &Usage,
    popup: &Popup,
) -> io::Result<Option<Applied>> {
    let Some(initial) = context.query() else {
        return Ok(None);
    };
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(viewport_height(popup)),
        },
    ) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            return Err(error);
        }
    };
    let result = run_loop(&mut terminal, catalog, context, initial, usage, popup);
    let _ = terminal.clear();
    drop(terminal);
    disable_raw_mode()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    catalog: &[Candidate],
    context: &Context,
    initial: &str,
    usage: &Usage,
    popup: &Popup,
) -> io::Result<Option<Applied>> {
    let mut query = initial.to_owned();
    let mut selected = 0_usize;
    loop {
        let suggestions = suggest_ranked(
            catalog,
            &query,
            popup
                .max_results
                .min(usize::from(popup.height.saturating_sub(4))),
            usage,
        );
        selected = if suggestions.is_empty() {
            0
        } else {
            selected.min(suggestions.len() - 1)
        };
        terminal.draw(|frame| {
            let colors = palette(popup.theme);
            let area = popup_rect(frame.area(), popup);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Block::default()
                    .title(" 🦀 Eita · autocomplete ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.background)),
                area,
            );
            let inner = area.inner(Margin::new(1, 1));
            let [header, list_area, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(inner);
            frame.render_widget(
                Paragraph::new(query.as_str())
                    .style(Style::default().fg(colors.border).bg(colors.background)),
                header,
            );
            let items: Vec<ListItem<'_>> = suggestions
                .iter()
                .map(|suggestion| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            suggestion.value.as_str(),
                            Style::default()
                                .fg(colors.text)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {}", suggestion.description),
                            Style::default().fg(colors.muted),
                        ),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(colors.highlight).fg(colors.text));
            let mut state = ListState::default();
            if !suggestions.is_empty() {
                state.select(Some(selected));
            }
            frame.render_stateful_widget(list, list_area, &mut state);
            frame.render_widget(
                Paragraph::new("↑↓ selecionar · ↵ aceitar · Esc")
                    .style(Style::default().fg(colors.muted).bg(colors.background)),
                footer,
            );
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter | KeyCode::Tab => {
                    return Ok(suggestions
                        .get(selected)
                        .map(|item| context.apply(&item.value)));
                }
                KeyCode::Up => selected = selected.saturating_sub(1),
                KeyCode::Down => {
                    if selected + 1 < suggestions.len() {
                        selected += 1;
                    }
                }
                KeyCode::Backspace => {
                    query.pop();
                    selected = 0;
                }
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None);
                }
                KeyCode::Char(character)
                    if !key
                        .modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    query.push(character);
                    selected = 0;
                }
                _ => {}
            }
        }
    }
}

/// A compact picker that is invoked only *after* the shell reports an unknown
/// command. This viewport appears in the scrollback above the next prompt.
pub fn recover_interactive(
    word: &str,
    corrections: &[crate::recover::Correction],
    popup: &Popup,
) -> io::Result<Option<String>> {
    if corrections.is_empty() {
        return Ok(None);
    }
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(viewport_height(popup)),
        },
    ) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            return Err(error);
        }
    };
    let result = recover_loop(&mut terminal, word, corrections, popup);
    let _ = terminal.clear();
    drop(terminal);
    disable_raw_mode()?;
    result
}

fn recover_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    word: &str,
    corrections: &[crate::recover::Correction],
    popup: &Popup,
) -> io::Result<Option<String>> {
    let mut selected = 0usize;
    loop {
        terminal.draw(|frame| {
            let colors = palette(popup.theme);
            let area = popup_rect(frame.area(), popup);
            frame.render_widget(Clear, area);
            frame.render_widget(
                Block::default()
                    .title(" 🦀 Eita · quis dizer? ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(colors.border))
                    .style(Style::default().bg(colors.background)),
                area,
            );
            let inner = area.inner(Margin::new(1, 1));
            let [header, rows, footer] = Layout::vertical([
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .areas(inner);
            frame.render_widget(
                Paragraph::new(format!("Não encontrei: {word}"))
                    .style(Style::default().fg(colors.border).bg(colors.background)),
                header,
            );
            let items: Vec<ListItem<'_>> = corrections
                .iter()
                .take(popup.max_results)
                .enumerate()
                .map(|(index, choice)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{}  {}", index + 1, choice.name),
                            Style::default()
                                .fg(colors.text)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  [{}]", choice.kind),
                            Style::default().fg(colors.muted),
                        ),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(colors.highlight).fg(colors.text));
            let mut state = ListState::default();
            state.select(Some(selected));
            frame.render_stateful_widget(list, rows, &mut state);
            frame.render_widget(
                Paragraph::new("↑↓ mover · ↵ editar · Esc sair")
                    .style(Style::default().fg(colors.muted).bg(colors.background)),
                footer,
            );
        })?;
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None);
                }
                KeyCode::Enter | KeyCode::Tab => {
                    return Ok(corrections.get(selected).map(|item| item.name.clone()));
                }
                KeyCode::Up => selected = selected.saturating_sub(1),
                KeyCode::Down => {
                    selected = (selected + 1).min(corrections.len().min(popup.max_results) - 1)
                }
                KeyCode::Char(digit @ '1'..='9') => {
                    let index = digit as usize - '1' as usize;
                    if let Some(item) = corrections.get(index) {
                        return Ok(Some(item.name.clone()));
                    }
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod popup_tests {
    use super::*;

    #[test]
    fn narrow_right_aligned_box_fits_terminal() {
        let popup = Popup {
            align: Align::Right,
            ..Popup::default()
        };
        assert_eq!(
            popup_rect(Rect::new(0, 0, 100, 10), &popup),
            Rect::new(52, 0, 48, 8)
        );
        assert_eq!(
            popup_rect(Rect::new(0, 0, 30, 6), &popup),
            Rect::new(0, 0, 30, 6)
        );
    }

    #[test]
    fn layout_placement_remains_within_reserved_rows() {
        let popup = Popup {
            position: Position::Below,
            ..Popup::default()
        };
        assert_eq!(viewport_height(&popup), 10);
        assert_eq!(popup_rect(Rect::new(0, 0, 80, 10), &popup).y, 2);
    }
}
