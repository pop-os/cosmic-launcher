use cosmic::iced::widget::image;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowKey {
    pub group: u32,
    pub id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailState {
    Placeholder,
    Loading,
    Ready(ThumbnailImage),
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThumbnailImageSource {
    ImageHandle(image::Handle),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailImage {
    pub window: WindowKey,
    pub source: ThumbnailImageSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailRequest {
    pub window: WindowKey,
    pub source: Option<RgbaThumbnailData>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailResult {
    pub window: WindowKey,
    pub state: ThumbnailState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaThumbnailData {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}
