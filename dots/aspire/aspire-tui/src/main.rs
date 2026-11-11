use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use std::io::{self, stdout};
use std::time::Duration;

const LOGO: &str = r#"
    █████╗ ███████╗██████╗ ██╗██████╗ ███████╗
   ██╔══██╗██╔════╝██╔══██╗██║██╔══██╗██╔════╝
   ███████║███████╗██████╔╝██║██████╔╝█████╗  
   ██╔══██║╚════██║██╔═══╝ ██║██╔══██╗██╔══╝  
   ██║  ██║███████║██║     ██║██║  ██║███████╗
   ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚═╝  ╚═╝╚══════╝
"#;

const TAGLINE: &str = "dotfiles · mangowm · aspire";

const ITEMS: &[&str] = &[
    "Apps",
    "Settings",
    "Style",
    "Utils",
    "System",
    "Extras",
];

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),
                    Constraint::Length(2),
                    Constraint::Min(6),
                    Constraint::Length(2),
                ])
                .split(area);

            // Logo
            let logo = Paragraph::new(LOGO)
                .alignment(Alignment::Center)
                .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(logo, chunks[0]);

            // Tagline
            let tag = Paragraph::new(TAGLINE)
                .alignment(Alignment::Center)
                .style(Style::default());
            f.render_widget(tag, chunks[1]);

            // Menu
            let items: Vec<ListItem> = ITEMS
                .iter()
                .map(|s| ListItem::new(format!("  {s}")))
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::NONE))
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
                .highlight_symbol("▸ ");
            f.render_stateful_widget(list, chunks[2], &mut state);

            // Footer
            let footer = Paragraph::new("↑/↓ navigate · Enter select · q quit")
                .alignment(Alignment::Center)
                .style(Style::default());
            f.render_widget(footer, chunks[3]);
        })?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Up => {
                            let i = state.selected().unwrap_or(0);
                            state.select(Some(if i == 0 { ITEMS.len() - 1 } else { i - 1 }));
                        }
                        KeyCode::Down => {
                            let i = state.selected().unwrap_or(0);
                            state.select(Some((i + 1) % ITEMS.len()));
                        }
                        KeyCode::Enter => {
                            if let Some(i) = state.selected() {
                                // placeholder action
                                let _ = ITEMS[i];
                            }
                        }
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn dump_frame() {
        let backend = TestBackend::new(90, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| {
            let area = f.area();
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(8),
                    Constraint::Length(2),
                    Constraint::Min(6),
                    Constraint::Length(2),
                ])
                .split(area);
            let logo = Paragraph::new(LOGO)
                .alignment(Alignment::Center)
                .style(Style::default().add_modifier(Modifier::BOLD));
            f.render_widget(logo, chunks[0]);
            let tag = Paragraph::new(TAGLINE)
                .alignment(Alignment::Center)
                .style(Style::default());
            f.render_widget(tag, chunks[1]);
            let items: Vec<ListItem> = ITEMS.iter().map(|s| ListItem::new(format!("  {s}"))).collect();
            let mut st = ListState::default();
            st.select(Some(0));
            let list = List::new(items)
                .highlight_style(Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD))
                .highlight_symbol("▸ ");
            f.render_stateful_widget(list, chunks[2], &mut st);
            let footer = Paragraph::new("↑/↓ navigate · Enter select · q quit")
                .alignment(Alignment::Center)
                .style(Style::default());
            f.render_widget(footer, chunks[3]);
        }).unwrap();
        println!("{}", terminal.backend().to_string());
    }
}
