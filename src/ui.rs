use crate::catalog::Candidate;
use crate::engine::suggest;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::{Terminal, TerminalOptions, Viewport};
use std::io;

pub fn pick(catalog: &[Candidate], initial: &str) -> io::Result<Option<String>> {
    enable_raw_mode()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = match Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(10),
        },
    ) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            return Err(error);
        }
    };
    let result = run_loop(&mut terminal, catalog, initial);
    let _ = terminal.clear();
    drop(terminal);
    disable_raw_mode()?;
    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    catalog: &[Candidate],
    initial: &str,
) -> io::Result<Option<String>> {
    let mut query = initial.to_owned();
    let mut selected = 0_usize;
    loop {
        let suggestions = suggest(catalog, &query, 6);
        selected = if suggestions.is_empty() {
            0
        } else {
            selected.min(suggestions.len() - 1)
        };
        terminal.draw(|frame| {
            let [header, list_area, footer] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(5),
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
                            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
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
            frame.render_widget("↑↓ navegar · Tab/Enter aceitar · Esc cancelar", footer);
        })?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => return Ok(None),
                KeyCode::Enter | KeyCode::Tab => {
                    return Ok(suggestions.get(selected).map(|item| item.value.clone()));
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
                    if !key.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    query.push(character);
                    selected = 0;
                }
                _ => {}
            }
        }
    }
}
