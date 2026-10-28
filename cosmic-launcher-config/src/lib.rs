use std::any::TypeId;
use std::borrow::Cow;

use cosmic::iced::Subscription;
use cosmic_config::CosmicConfigEntry;
use cosmic_config::cosmic_config_derive::CosmicConfigEntry;
use serde::{Deserialize, Serialize};

pub const APP_ID: &str = "com.system76.CosmicLauncher";
pub const CONFIG_VERSION: u64 = 1;

pub enum Message {
    TogglePlugin(String, bool),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Hash)]
pub enum IconSource {
    Name(Cow<'static, str>),
    Mime(Cow<'static, str>),
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, Hash)]
pub struct PluginConfig {
    pub name: String,
    pub description: String,
    pub icon: Option<IconSource>,
    pub disabled: bool,
}

impl From<pop_launcher::PluginResult> for PluginConfig {
    fn from(plugin: pop_launcher::PluginResult) -> Self {
        Self {
            name: plugin.name,
            description: plugin.description,
            icon: plugin.icon.map(|icon| match icon {
                pop_launcher::IconSource::Name(cow) => IconSource::Name(cow),
                pop_launcher::IconSource::Mime(cow) => IconSource::Mime(cow),
            }),
            disabled: plugin.disabled,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, CosmicConfigEntry, Deserialize, Serialize, Hash)]
#[serde(deny_unknown_fields)]
#[version = 1]
#[id = "com.system76.CosmicLauncher"]
pub struct Config {
    pub plugins: Vec<PluginConfig>,
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
