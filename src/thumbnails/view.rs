use cosmic::Element;
use cosmic::iced::{Border, Color, Length, Shadow, widget::image};
use cosmic::theme::Container;
use cosmic::widget::space::horizontal as horizontal_space;
use cosmic::widget::{container, row, text};

use crate::thumbnails::model::{ThumbnailImage, ThumbnailImageSource};
use crate::thumbnails::{THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH, ThumbnailState};

pub fn thumbnail_placeholder<'a, Message: 'a>() -> Element<'a, Message> {
    container(horizontal_space().width(Length::Fill))
        .width(Length::Fixed(THUMBNAIL_WIDTH as f32))
        .height(Length::Fixed(THUMBNAIL_HEIGHT as f32))
        .class(Container::Custom(Box::new(|theme| {
            let t = theme.cosmic();

            container::Style {
                background: Some(Color::from_rgba(0.2, 0.2, 0.2, 0.8).into()),
                border: Border {
                    radius: t.radius_s().into(),
                    width: 1.0,
                    color: t.bg_divider().into(),
                },
                text_color: None,
                icon_color: None,
                shadow: Shadow::default(),
                snap: true,
            }
        })))
        .into()
}

pub fn thumbnail_loading<'a, Message: 'a>() -> Element<'a, Message> {
    container(row![text::caption("Loading...")].align_y(cosmic::iced::Alignment::Center))
        .width(Length::Fixed(THUMBNAIL_WIDTH as f32))
        .height(Length::Fixed(THUMBNAIL_HEIGHT as f32))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .class(Container::Custom(Box::new(|theme| {
            let t = theme.cosmic();

            container::Style {
                background: Some(Color::from_rgba(0.8, 0.5, 0.1, 0.8).into()),
                border: Border {
                    radius: t.radius_s().into(),
                    width: 1.0,
                    color: t.bg_divider().into(),
                },
                text_color: Some(t.on_bg_color().into()),
                icon_color: None,
                shadow: Shadow::default(),
                snap: true,
            }
        })))
        .into()
}

pub fn thumbnail_ready<'a, Message: 'a>(thumbnail_image: ThumbnailImage) -> Element<'a, Message> {
    match &thumbnail_image.source {
        ThumbnailImageSource::ImageHandle(handle) => container(
            image(handle.clone())
                .width(Length::Fixed(THUMBNAIL_WIDTH as f32))
                .height(Length::Fixed(THUMBNAIL_HEIGHT as f32)),
        )
        .width(Length::Fixed(THUMBNAIL_WIDTH as f32))
        .height(Length::Fixed(THUMBNAIL_HEIGHT as f32))
        .into(),
    }
}

pub fn thumbnail_view<'a, Message: 'a>(state: ThumbnailState) -> Option<Element<'a, Message>> {
    match state {
        ThumbnailState::Placeholder => Some(thumbnail_placeholder()),
        ThumbnailState::Loading => Some(thumbnail_loading()),
        ThumbnailState::Ready(image) => Some(thumbnail_ready(image)),
        ThumbnailState::Unavailable => Some(thumbnail_placeholder()),
    }
}
