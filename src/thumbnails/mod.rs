pub mod loader;
pub mod model;
pub mod provider;
pub mod view;

pub use model::{ThumbnailRequest, ThumbnailState};

pub const THUMBNAIL_WIDTH: u32 = 160;
pub const THUMBNAIL_HEIGHT: u32 = 90;
