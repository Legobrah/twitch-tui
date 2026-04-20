use ratatui::style::Color;

pub const CYAN: Color = Color::Rgb(0, 212, 255);
pub const BG: Color = Color::Rgb(6, 8, 22);
pub const TEXT: Color = Color::Rgb(205, 214, 244);
pub const DIM_TEXT: Color = Color::Rgb(147, 153, 178);
pub const RED: Color = Color::Rgb(243, 139, 168);
pub const GREEN: Color = Color::Rgb(166, 227, 161);
pub const YELLOW: Color = Color::Rgb(249, 226, 175);
pub const BORDER: Color = Color::Rgb(30, 40, 60);
pub const SURFACE: Color = Color::Rgb(14, 17, 34);
pub const SELECTION_BG: Color = Color::Rgb(15, 20, 40);
pub const STATUS_BG: Color = Color::Rgb(10, 14, 30);
pub const MENTION_BG: Color = Color::Rgb(40, 24, 16);
pub const MENTION_FG: Color = Color::Rgb(250, 179, 135);

pub const LIVE_DOT: &str = "\u{25cf}";
pub const OFFLINE_DOT: &str = "\u{25cb}";
pub const POINTER: &str = "\u{25b8}";

pub const SPINNER_FRAMES: &[&str] = &[
    "\u{280b}", "\u{2819}", "\u{2839}", "\u{2838}",
    "\u{283c}", "\u{2834}", "\u{2826}", "\u{2827}",
    "\u{2807}", "\u{280f}",
];
