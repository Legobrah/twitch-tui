use crate::db::SavedChannel;
use crate::twitch::{Channel, ChatMessage, Game, Vod};

#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    SavedChannels,
    Categories,
    CategoryStreams {
        game_id: String,
        game_name: String,
    },
    Search {
        query: String,
    },
    Vods {
        channel_name: String,
    },
    Followed,
    QualitySelect {
        channel_name: String,
        channel_display_name: String,
        quality_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Browse,
    Detail,
    Chat,
}

#[derive(Debug)]
pub enum AppEvent {
    ChannelsLoaded(Vec<Channel>, Option<String>),
    CategoriesLoaded(Vec<Game>, Option<String>),
    CategoryStreamsLoaded(Vec<Channel>, Option<String>),
    SearchResults(Vec<Channel>, Option<String>),
    VodsLoaded(Vec<Vod>, Option<String>),
    ChatMessage(ChatMessage),
    ChatConnected(String),
    Error(String),
    Tick,
}

pub const QUALITY_OPTIONS: &[&str] = &["best", "1080p", "720p", "480p", "360p", "audio_only"];

pub struct App {
    pub mode: AppMode,
    pub focus: FocusTarget,
    pub saved_channels: Vec<SavedChannel>,
    pub channels: Vec<Channel>,
    pub categories: Vec<Game>,
    pub category_streams: Vec<Channel>,
    pub search_results: Vec<Channel>,
    pub vods: Vec<Vod>,
    pub chat_messages: Vec<ChatMessage>,
    pub chat_input: String,
    pub selected_index: usize,
    pub show_help: bool,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub should_quit: bool,
    pub pagination_cursor: Option<String>,
    pub error_time: Option<std::time::Instant>,
}

impl App {
    pub fn new(saved_channels: Vec<SavedChannel>) -> Self {
        Self {
            mode: AppMode::SavedChannels,
            focus: FocusTarget::Browse,
            saved_channels,
            channels: Vec::new(),
            categories: Vec::new(),
            category_streams: Vec::new(),
            search_results: Vec::new(),
            vods: Vec::new(),
            chat_messages: Vec::new(),
            chat_input: String::new(),
            selected_index: 0,
            show_help: false,
            error_message: None,
            is_loading: false,
            should_quit: false,
            pagination_cursor: None,
            error_time: None,
        }
    }

    pub fn current_channels(&self) -> &[Channel] {
        match &self.mode {
            AppMode::SavedChannels | AppMode::Followed => &self.channels,
            AppMode::CategoryStreams { .. } => &self.category_streams,
            AppMode::Search { .. } => &self.search_results,
            _ => &self.channels,
        }
    }

    pub fn selected_channel(&self) -> Option<&Channel> {
        self.current_channels().get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        let len = self.current_channels().len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    pub fn select_prev(&mut self) {
        let len = self.current_channels().len();
        if len > 0 {
            self.selected_index = (self.selected_index + len - 1) % len;
        }
    }

    pub fn cycle_focus(&mut self) {
        self.focus = match self.focus {
            FocusTarget::Browse => FocusTarget::Detail,
            FocusTarget::Detail => FocusTarget::Chat,
            FocusTarget::Chat => FocusTarget::Browse,
        };
    }

    pub fn reset_selection(&mut self) {
        self.selected_index = 0;
    }

    pub fn mode_label(&self) -> &str {
        match &self.mode {
            AppMode::SavedChannels => "Saved Channels",
            AppMode::Categories => "Categories",
            AppMode::CategoryStreams { game_name, .. } => game_name,
            AppMode::Search { .. } => "Search",
            AppMode::Vods { channel_name } => channel_name,
            AppMode::Followed => "Following",
            AppMode::QualitySelect { .. } => "Quality",
        }
    }

    pub fn key_hints(&self) -> &str {
        if self.show_help {
            return "? close help";
        }
        if let FocusTarget::Chat = self.focus {
            return "type message · Enter send · Esc back";
        }
        match &self.mode {
            AppMode::Search { .. } => "type to search · Esc back · Enter watch",
            AppMode::Categories => "Enter select · Esc back · ? help",
            AppMode::Vods { .. } => "Enter play · Esc back · ? help",
            AppMode::QualitySelect { .. } => "j/k choose · Enter select · Esc cancel",
            _ => "j/k nav · Enter watch · s save · n more · ? help",
        }
    }
}
