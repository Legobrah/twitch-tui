use crate::db::SavedChannel;
use crate::thumb::ThumbnailCache;
use crate::twitch::{Channel, ChatMessage, Game, Vod};
use image::DynamicImage;
use ratatui_image::picker::Picker;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

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
    ThumbnailReady(String, DynamicImage),
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
    pub chat_history: Vec<String>,
    pub chat_history_index: Option<usize>,
    pub selected_index: usize,
    pub show_help: bool,
    pub help_scroll: u16,
    pub help_max_scroll: u16,
    pub error_message: Option<String>,
    pub is_loading: bool,
    pub should_quit: bool,
    pub pagination_cursor: Option<String>,
    pub watching_channel: Option<Channel>,
    pub error_time: Option<std::time::Instant>,
    pub search_seq: Arc<AtomicU64>,
    pub username: Option<String>,
    pub has_oauth: bool,
    pub spinner_frame: usize,
    pub picker: Option<Picker>,
    pub thumb_cache: ThumbnailCache,
    pub thumb_seq: Arc<AtomicU64>,
    pub last_thumb_key: Option<String>,
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
            chat_history: Vec::new(),
            chat_history_index: None,
            selected_index: 0,
            show_help: false,
            help_scroll: 0,
            help_max_scroll: 0,
            error_message: None,
            is_loading: false,
            should_quit: false,
            pagination_cursor: None,
            watching_channel: None,
            error_time: None,
            search_seq: Arc::new(AtomicU64::new(0)),
            username: None,
            has_oauth: false,
            spinner_frame: 0,
            picker: None,
            thumb_cache: ThumbnailCache::new(),
            thumb_seq: Arc::new(AtomicU64::new(0)),
            last_thumb_key: None,
        }
    }

    pub fn clamp_selection(&mut self) {
        let len = self.current_list_len();
        if len == 0 {
            self.selected_index = 0;
        } else if self.selected_index >= len {
            self.selected_index = len - 1;
        }
    }

    pub fn jump_top(&mut self) {
        self.selected_index = 0;
    }

    pub fn jump_bottom(&mut self) {
        let len = self.current_list_len();
        self.selected_index = len.saturating_sub(1);
    }

    pub fn page_down(&mut self, page: usize) {
        let len = self.current_list_len();
        if len == 0 {
            return;
        }
        self.selected_index = (self.selected_index + page).min(len - 1);
    }

    pub fn page_up(&mut self, page: usize) {
        self.selected_index = self.selected_index.saturating_sub(page);
    }

    pub fn current_channels(&self) -> &[Channel] {
        match &self.mode {
            AppMode::SavedChannels | AppMode::Followed => &self.channels,
            AppMode::CategoryStreams { .. } => &self.category_streams,
            AppMode::Search { .. } => &self.search_results,
            _ => &self.channels,
        }
    }

    pub fn current_list_len(&self) -> usize {
        match &self.mode {
            AppMode::SavedChannels | AppMode::Followed => self.channels.len(),
            AppMode::Categories => self.categories.len(),
            AppMode::CategoryStreams { .. } => self.category_streams.len(),
            AppMode::Search { .. } => self.search_results.len(),
            AppMode::Vods { .. } => self.vods.len(),
            AppMode::QualitySelect { .. } => QUALITY_OPTIONS.len(),
        }
    }

    pub fn selected_channel(&self) -> Option<&Channel> {
        self.current_channels().get(self.selected_index)
    }

    pub fn select_next(&mut self) {
        let len = self.current_list_len();
        if len > 0 {
            self.selected_index = (self.selected_index + 1) % len;
        }
    }

    pub fn select_prev(&mut self) {
        let len = self.current_list_len();
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
            return "j/k scroll · ? close";
        }
        if let FocusTarget::Chat = self.focus {
            return "Enter send · ↑/↓ history · Esc back";
        }
        match &self.mode {
            AppMode::Search { .. } => "type to search · Enter watch · Esc back",
            AppMode::Categories => "Enter select · r refresh · Esc back",
            AppMode::Vods { .. } => "Enter play · r refresh · Esc back",
            AppMode::QualitySelect { .. } => "j/k choose · Enter select · Esc default",
            _ => "Enter watch · s save · r refresh · n more · ? help",
        }
    }
}
