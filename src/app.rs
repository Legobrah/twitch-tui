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
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusTarget {
    Browse,
    Detail,
    Chat,
}

#[derive(Debug)]
pub enum AppEvent {
    ChannelsLoaded(Vec<Channel>),
    CategoriesLoaded(Vec<Game>),
    CategoryStreamsLoaded(Vec<Channel>),
    SearchResults(Vec<Channel>),
    VodsLoaded(Vec<Vod>),
    ChatMessage(ChatMessage),
    ChatConnected(String),
    Error(String),
    Tick,
}

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
        if len > 0 && self.selected_index < len - 1 {
            self.selected_index += 1;
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
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
}
