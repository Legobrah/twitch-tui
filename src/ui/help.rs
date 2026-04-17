use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use super::theme;

pub fn render(f: &mut Frame, area: Rect) {
    let help_text = "\
Navigation
  Tab/Shift+Tab  Switch pane
  j/k, Up/Down   Navigate list
  n              Load more results
  Esc            Back to saved channels

Actions
  Enter          Watch stream (quality picker)
  s              Save/unsave channel
  /              Search channels
  c              Categories view
  v              VODs (selected channel)
  f              Followed channels
  r              Refresh
  q              Quit

Chat pane
  Tab            Switch to chat
  type to compose, Enter to send
  Esc            Back to browse

Quality picker
  j/k or Up/Down  Select quality
  Enter           Confirm selection
  Esc             Use default quality

?               Toggle this help";
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
