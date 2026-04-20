use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use image::DynamicImage;
use ratatui_image::picker::Picker;
use ratatui_image::protocol::StatefulProtocol;
use tokio::sync::mpsc::UnboundedSender;
use tracing::{debug, warn};

use crate::app::AppEvent;
use crate::twitch::Channel;

/// Pixel dimensions we request from Twitch (16:9). The widget downsamples
/// to the actual rendered size, so fetching a little larger than needed
/// preserves quality across terminal sizes.
const THUMB_W: u32 = 640;
const THUMB_H: u32 = 360;

const DEBOUNCE_MS: u64 = 150;
const FETCH_TIMEOUT_SECS: u64 = 5;
const CACHE_CAPACITY: usize = 32;

pub struct ThumbnailCache {
    entries: HashMap<String, StatefulProtocol>,
    order: VecDeque<String>,
}

impl ThumbnailCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn contains(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut StatefulProtocol> {
        self.entries.get_mut(key)
    }

    pub fn insert(&mut self, key: String, proto: StatefulProtocol) {
        if self.entries.contains_key(&key) {
            self.entries.insert(key, proto);
            return;
        }
        if self.order.len() >= CACHE_CAPACITY {
            if let Some(evict) = self.order.pop_front() {
                self.entries.remove(&evict);
            }
        }
        self.order.push_back(key.clone());
        self.entries.insert(key, proto);
    }
}

impl Default for ThumbnailCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache key for a channel: id + stream start timestamp, so a new broadcast
/// invalidates the old thumbnail.
pub fn cache_key(channel: &Channel) -> Option<String> {
    if !channel.is_live {
        return None;
    }
    Some(format!(
        "{}@{}",
        channel.twitch_id,
        channel.started_at.as_deref().unwrap_or("live")
    ))
}

/// Substitute Twitch's {width}/{height} placeholders in the thumbnail URL.
fn resolve_url(raw: &str) -> Option<String> {
    if raw.is_empty() || !raw.starts_with("http") {
        return None;
    }
    Some(
        raw.replace("{width}", &THUMB_W.to_string())
            .replace("{height}", &THUMB_H.to_string()),
    )
}

/// Spawn a debounced fetch. If `seq` is bumped by a newer request before the
/// debounce or fetch completes, the task exits without emitting an event.
pub fn spawn_fetch(
    seq: Arc<AtomicU64>,
    tx: UnboundedSender<AppEvent>,
    key: String,
    raw_url: String,
) {
    let my_seq = seq.fetch_add(1, Ordering::SeqCst) + 1;
    let Some(url) = resolve_url(&raw_url) else {
        return;
    };
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(DEBOUNCE_MS)).await;
        if seq.load(Ordering::SeqCst) != my_seq {
            return;
        }
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(FETCH_TIMEOUT_SECS))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                warn!("Thumbnail client build failed: {}", e);
                return;
            }
        };
        let bytes = match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => match r.bytes().await {
                Ok(b) => b,
                Err(e) => {
                    debug!("Thumbnail body read failed: {}", e);
                    return;
                }
            },
            Ok(r) => {
                debug!("Thumbnail HTTP {}: {}", r.status(), url);
                return;
            }
            Err(e) => {
                debug!("Thumbnail fetch error: {}", e);
                return;
            }
        };
        if seq.load(Ordering::SeqCst) != my_seq {
            return;
        }
        let img = match tokio::task::spawn_blocking(move || image::load_from_memory(&bytes))
            .await
        {
            Ok(Ok(img)) => img,
            Ok(Err(e)) => {
                debug!("Thumbnail decode failed: {}", e);
                return;
            }
            Err(e) => {
                debug!("Thumbnail decode join failed: {}", e);
                return;
            }
        };
        if seq.load(Ordering::SeqCst) != my_seq {
            return;
        }
        let _ = tx.send(AppEvent::ThumbnailReady(key, img));
    });
}

/// Build a StatefulProtocol from a decoded image. Must run on the main thread
/// because the protocol holds terminal-facing state.
pub fn build_protocol(picker: &Picker, img: DynamicImage) -> StatefulProtocol {
    picker.new_resize_protocol(img)
}
