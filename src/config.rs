use cosmic::{
    cosmic_config::cosmic_config_derive::CosmicConfigEntry,
    cosmic_config::{self, CosmicConfigEntry},
    iced::Subscription,
};
use serde::{Deserialize, Serialize};
use std::any::TypeId;
use tracing::{error, info};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONFIG_VERSION: u64 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, CosmicConfigEntry)]
#[serde(default)]
pub struct Config {
    pub anchor: Anchor,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    #[default]
    Top,
    Center,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            anchor: Anchor::default(),
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

impl From<Anchor> for cosmic::iced_winit::commands::layer_surface::Anchor {
    fn from(pos: Anchor) -> Self {
        match pos {
            Anchor::Top => cosmic::iced_winit::commands::layer_surface::Anchor::TOP,
            Anchor::Center => cosmic::iced_winit::commands::layer_surface::Anchor::empty(),
        }
    }
}

pub fn profile() -> &'static str {
    std::env!("OUT_DIR")
        .split(std::path::MAIN_SEPARATOR)
        .nth_back(3)
        .unwrap_or("unknown")
}
