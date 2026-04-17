use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, area: Rect) {
    let help_text = "\
Tab/Shift+Tab  Switch pane
j/k, Up/Down    Navigate list
Enter           Watch stream
s               Save/unsave channel
/               Search channels
c               Categories view
v               VODs (selected channel)
f               Followed channels
r               Refresh
Escape          Back to saved
?               This help
q               Quit

Chat pane: type to compose, Enter to send";
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help (? to close) ")
        .style(Style::default().fg(theme::CYAN));
    let para = Paragraph::new(help_text)
        .block(block)
        .style(
            Style::default()
                .fg(theme::TEXT)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(Clear, area);
    f.render_widget(para, area);
}
