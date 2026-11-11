// can't access private; duplicate minimal render instead
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

fn main() {
    let backend = TestBackend::new(90, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Length(2), Constraint::Min(6), Constraint::Length(2)])
            .split(area);
        // logo placeholder
const LOGO: &str = r#"
    █████╗ ███████╗██████╗ ██╗██████╗ ███████╗
   ██╔══██╗██╔════╝██╔══██╗██║██╔══██╗██╔════╝
   ███████║███████╗██████╔╝██║██████╔╝█████╗  
   ██╔══██║╚════██║██╔═══╝ ██║██╔══██╗██╔══╝  
   ██║  ██║███████║██║     ██║██║  ██║███████╗
   ╚═╝  ╚═╝╚══════╝╚═╝     ╚═╝╚═╝  ╚═╝╚══════╝
"#;
        let logo = Paragraph::new(LOGO).alignment(Alignment::Center);
        f.render_widget(logo, chunks[0]);
        let tag = Paragraph::new("dotfiles · mangowm · aspire").alignment(Alignment::Center);
        f.render_widget(tag, chunks[1]);
        let items = vec!["Apps","Settings","Style","Utils","System","Extras"];
        let list_items: Vec<ListItem> = items.iter().map(|s| ListItem::new(format!("  {s}"))).collect();
        let mut st = ListState::default();
        st.select(Some(0));
        let list = List::new(list_items).highlight_symbol("▸ ");
        f.render_stateful_widget(list, chunks[2], &mut st);
        let footer = Paragraph::new("↑/↓ navigate · Enter select · q quit").alignment(Alignment::Center);
        f.render_widget(footer, chunks[3]);
    }).unwrap();
    println!("{}", terminal.backend().to_string());
}
