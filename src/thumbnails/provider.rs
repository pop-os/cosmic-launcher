use std::{
    collections::{HashMap, HashSet},
    hash::{DefaultHasher, Hash, Hasher},
};

use pop_launcher::SearchResult;

use crate::thumbnails::{
    ThumbnailRequest, ThumbnailState,
    model::{RgbaThumbnailData, WindowKey},
};

#[derive(Debug, Clone)]
struct CachedThumbnail {
    signature: Option<ThumbnailSignature>,
    state: ThumbnailState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThumbnailSignature {
    width: u32,
    height: u32,
    len: usize,
    hash: u64,
}

#[derive(Debug, Default, Clone)]
pub struct ThumbnailProvider {
    cache: HashMap<WindowKey, CachedThumbnail>,
}

impl ThumbnailProvider {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    fn thumbnail_signature(item: &SearchResult) -> Option<ThumbnailSignature> {
        let thumbnail = item.thumbnail.as_ref()?;

        let mut hasher = DefaultHasher::new();
        thumbnail.pixels.hash(&mut hasher);

        Some(ThumbnailSignature {
            width: thumbnail.width,
            height: thumbnail.height,
            len: thumbnail.pixels.len(),
            hash: hasher.finish(),
        })
    }

    pub fn thumbnail_state(&self, item: &SearchResult) -> ThumbnailState {
        let Some(window_key) = Self::window_key(item) else {
            return ThumbnailState::Unavailable;
        };

        self.cache
            .get(&window_key)
            .map(|cached| cached.state.clone())
            .unwrap_or(ThumbnailState::Placeholder)
    }

    pub fn request_thumbnail(&mut self, item: &SearchResult) -> Option<ThumbnailRequest> {
        let window_key = Self::window_key(item)?;
        let signature = Self::thumbnail_signature(item);

        let should_request = match self.cache.get(&window_key) {
            Some(CachedThumbnail {
                signature: cached_signature,
                state: ThumbnailState::Ready(_) | ThumbnailState::Loading,
            }) if *cached_signature == signature => false,

            _ => true,
        };

        if !should_request {
            return None;
        }

        self.cache.insert(
            window_key,
            CachedThumbnail {
                signature,
                state: ThumbnailState::Loading,
            },
        );

        Some(ThumbnailRequest {
            window: window_key,
            source: item.thumbnail.as_ref().map(|thumbnail| RgbaThumbnailData {
                width: thumbnail.width,
                height: thumbnail.height,
                pixels: thumbnail.pixels.clone(),
            }),
        })
    }

    pub fn set_thumbnail_state(&mut self, window_key: WindowKey, state: ThumbnailState) {
        if let Some(cached) = self.cache.get_mut(&window_key) {
            cached.state = state;
        } else {
            self.cache.insert(
                window_key,
                CachedThumbnail {
                    signature: None,
                    state,
                },
            );
        }
    }

    pub fn window_key(item: &SearchResult) -> Option<WindowKey> {
        item.window.map(|(group, id)| WindowKey { group, id })
    }

    pub fn retain_visible_windows<'a>(&mut self, items: impl Iterator<Item = &'a SearchResult>) {
        let visible_windows: HashSet<_> = items.filter_map(Self::window_key).collect();

        self.cache
            .retain(|window_key, _| visible_windows.contains(window_key));
    }
}
