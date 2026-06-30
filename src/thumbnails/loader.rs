use cosmic::iced::widget::image;

use crate::thumbnails::{
    ThumbnailRequest, ThumbnailState,
    model::{ThumbnailImage, ThumbnailImageSource, ThumbnailResult},
};

async fn load_source(request: &ThumbnailRequest) -> Option<ThumbnailImageSource> {
    request.source.as_ref().map(|source| {
        ThumbnailImageSource::ImageHandle(image::Handle::from_rgba(
            source.width,
            source.height,
            source.pixels.clone(),
        ))
    })
}

pub async fn load_thumbnail(request: ThumbnailRequest) -> ThumbnailResult {
    let state = match load_source(&request).await {
        Some(source) => ThumbnailState::Ready(ThumbnailImage {
            window: request.window,
            source,
        }),

        None => ThumbnailState::Unavailable,
    };

    ThumbnailResult {
        window: request.window,
        state: state,
    }
}
