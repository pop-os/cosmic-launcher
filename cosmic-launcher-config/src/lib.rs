use std::any::TypeId;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use cosmic::iced::Subscription;
use cosmic_config::CosmicConfigEntry;
use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "com.system76.CosmicLauncher";
pub const CONFIG_VERSION: u64 = 1;

#[derive(Debug, Clone, Default, PartialEq, CosmicConfigEntry, Deserialize, Serialize, Hash)]
#[serde(deny_unknown_fields)]
#[version = 1]
#[id = "com.system76.CosmicLauncher"]
pub struct Config {
    pub disabled_plugins: BTreeSet<String>,
}

impl Config {
    pub fn load() -> (Self, Option<cosmic_config::Config>) {
        match cosmic_config::Config::new(APP_ID, CONFIG_VERSION) {
            Ok(handler) => {
                let config = Config::get_entry(&handler)
                    .inspect_err(|(errors, _)| {
                        for err in errors {
                            log::error!("{err}")
                        }
                    })
                    .unwrap_or_else(|(_, config)| config);
                (config, Some(handler))
            }
            Err(e) => {
                log::error!("Failed to get settings for `{APP_ID}` (v {CONFIG_VERSION}): {e}");
                (Config::default(), None)
            }
        }
    }

    pub fn subscription() -> Subscription<cosmic_config::Update<Self>> {
        struct ConfigSubscription;
        cosmic_config::config_subscription(
            TypeId::of::<ConfigSubscription>(),
            APP_ID.into(),
            CONFIG_VERSION,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash)]
pub enum IconSource {
    Name(Cow<'static, str>),
    Mime(Cow<'static, str>),
}

impl From<pop_launcher::IconSource> for IconSource {
    fn from(icon: pop_launcher::IconSource) -> Self {
        match icon {
            pop_launcher::IconSource::Name(cow) => Self::Name(cow),
            pop_launcher::IconSource::Mime(cow) => Self::Mime(cow),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, Hash)]
pub struct PluginState {
    pub description: String,
    pub icon: Option<IconSource>,
}

#[derive(Debug, Clone, Default, PartialEq, CosmicConfigEntry, Deserialize, Serialize, Hash)]
#[serde(deny_unknown_fields)]
#[version = 1]
#[id = "com.system76.CosmicLauncher"]
pub struct State {
    pub plugins: BTreeMap<String, PluginState>,
}

impl State {
    pub fn load() -> (Self, Option<cosmic_config::Config>) {
        match cosmic_config::Config::new_state(APP_ID, CONFIG_VERSION) {
            Ok(handler) => {
                let config = State::get_entry(&handler)
                    .inspect_err(|(errors, _)| {
                        for err in errors {
                            log::error!("{err}")
                        }
                    })
                    .unwrap_or_else(|(_, config)| config);
                (config, Some(handler))
            }
            Err(e) => {
                log::error!("Failed to get settings for `{APP_ID}` (v {CONFIG_VERSION}): {e}");
                (State::default(), None)
            }
        }
    }
}
