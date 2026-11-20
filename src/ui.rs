use crate::catalog::Candidate;
use crate::context::{Applied, Context};
use crate::engine::suggest_ranked;
use crate::usage::Usage;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io;

pub fn pick(
    catalog: &[Candidate],
    context: &Context,
    usage: &Usage,
    height: u16,
) -> io::Result<Option<Applied>> {
    let Some(initial) = context.query() else {
        return Ok(None);
    };
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    ) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            return Err(error);
        }
    };
    let result = run_loop(&mut terminal, catalog, context, initial, usage);
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
) -> io::Result<Option<Applied>> {
    let mut query = initial.to_owned();
    let mut selected = 0_usize;
    loop {
        let suggestions = suggest_ranked(catalog, &query, 6, usage);
        selected = if suggestions.is_empty() {
            0
        } else {
            selected.min(suggestions.len() - 1)
        };
        terminal.draw(|frame| {
            let [header, list_area, footer] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(2),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            let input = Paragraph::new(query.as_str())
                .block(Block::default().title(" 🦀 Eita ").borders(Borders::ALL))
                .style(Style::default().fg(Color::Cyan));
            frame.render_widget(input, header);
            let items: Vec<ListItem<'_>> = suggestions
                .iter()
                .map(|suggestion| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            suggestion.value.as_str(),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  {}", suggestion.description),
                            Style::default().fg(Color::Gray),
                        ),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(Color::Blue));
            let mut state = ListState::default();
            if !suggestions.is_empty() {
                state.select(Some(selected));
            }
            frame.render_stateful_widget(list, list_area, &mut state);
            frame.render_widget("↑↓ mover · Enter aceitar · Esc fechar", footer);
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
    height: u16,
) -> io::Result<Option<String>> {
    if corrections.is_empty() {
        return Ok(None);
    }
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(height),
        },
    ) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            return Err(error);
        }
    };
    let result = recover_loop(&mut terminal, word, corrections);
    let _ = terminal.clear();
    drop(terminal);
    disable_raw_mode()?;
    result
}

fn recover_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    word: &str,
    corrections: &[crate::recover::Correction],
) -> io::Result<Option<String>> {
    let mut selected = 0usize;
    loop {
        terminal.draw(|frame| {
            let [header, rows, footer] = Layout::vertical([
                Constraint::Length(2),
                Constraint::Min(2),
                Constraint::Length(1),
            ])
            .areas(frame.area());
            let title = Paragraph::new(format!("🦀 Eita: '{word}' não existe. Quis dizer?")).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
            frame.render_widget(title, header);
            let items: Vec<ListItem<'_>> = corrections
                .iter()
                .enumerate()
                .map(|(index, choice)| {
                    ListItem::new(Line::from(vec![
                        Span::styled(
                            format!("{}  {}", index + 1, choice.name),
                            Style::default()
                                .fg(Color::White)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("  [{}] {}", choice.kind, choice.description),
                            Style::default().fg(Color::Gray),
                        ),
                    ]))
                })
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::TOP | Borders::BOTTOM))
                .highlight_symbol("› ")
                .highlight_style(Style::default().bg(Color::Blue));
            let mut state = ListState::default();
            state.select(Some(selected));
            frame.render_stateful_widget(list, rows, &mut state);
            frame.render_widget(
                "↑↓ selecionar · 1-5 escolha · Enter editar · Esc sair",
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
                KeyCode::Down => selected = (selected + 1).min(corrections.len() - 1),
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
