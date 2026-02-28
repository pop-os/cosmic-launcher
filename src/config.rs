use cosmic::{
    cosmic_config::cosmic_config_derive::CosmicConfigEntry,
    cosmic_config::{self, CosmicConfigEntry},
    iced::Subscription,
    iced_winit::commands::layer_surface::Anchor,
};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use tracing::{error, info};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, CosmicConfigEntry)]
#[serde(default)]
pub struct Config {
    pub anchor_position: AnchorPosition,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AnchorPosition {
    Top,
    Center,
    Bottom,
}

impl Default for AnchorPosition {
    fn default() -> Self {
        AnchorPosition::Top
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            anchor_position: AnchorPosition::default(),
        }
    }
}

impl Config {
    pub fn load() -> (Option<cosmic_config::Config>, Self) {
        match cosmic_config::Config::new(
            <crate::app::CosmicLauncher as cosmic::Application>::APP_ID,
            CONFIG_VERSION,
        ) {
            Ok(config_handler) => {
                let config = match Self::get_entry(&config_handler) {
                    Ok(ok) => ok,
                    Err((errs, config)) => {
                        info!("errors loading config: {errs:?}");
                        config
                    }
                };
                (Some(config_handler), config)
            }
            Err(err) => {
                error!("failed to create config handler: {err}");
                (None, Self::default())
            }
        }
    }

    pub fn subscription() -> Subscription<cosmic_config::Update<Self>> {
        struct ConfigSubscription;
        cosmic_config::config_subscription(
            TypeId::of::<ConfigSubscription>(),
            <crate::app::CosmicLauncher as cosmic::Application>::APP_ID.into(),
            CONFIG_VERSION,
        )
    }
}

impl From<AnchorPosition> for Anchor {
    fn from(pos: AnchorPosition) -> Self {
        match pos {
            AnchorPosition::Top => Anchor::TOP,
            AnchorPosition::Center => Anchor::empty(),
            AnchorPosition::Bottom => Anchor::BOTTOM,
        }
    }
}

pub fn profile() -> &'static str {
    std::env!("OUT_DIR")
        .split(std::path::MAIN_SEPARATOR)
        .nth_back(3)
        .unwrap_or("unknown")
}
