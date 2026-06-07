use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use pop_launcher::WorkspaceFilter;
use serde::{Deserialize, Serialize};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const CONFIG_VERSION: u64 = 1;
pub const APP_ID: &str = "com.system76.CosmicLauncher";

pub fn profile() -> &'static str {
    std::env!("OUT_DIR")
        .split(std::path::MAIN_SEPARATOR)
        .nth_back(3)
        .unwrap_or("unknown")
}

/// Which workspaces the window switcher should include.
#[derive(Debug, Clone, Copy, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceScope {
    /// List windows from every workspace.
    #[default]
    All,
    /// List only windows on the currently active workspace(s).
    Current,
}

impl WorkspaceScope {
    pub const fn to_filter(self) -> WorkspaceFilter {
        match self {
            Self::All => WorkspaceFilter::All,
            Self::Current => WorkspaceFilter::Current,
        }
    }
}

/// Window switcher defaults for `cosmic-launcher alt-tab`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, CosmicConfigEntry)]
#[version = 1]
pub struct WindowSwitcher {
    /// Default scope for `cosmic-launcher alt-tab` / `shift-alt-tab`.
    #[serde(default)]
    pub default_scope: WorkspaceScope,
}

impl Default for WindowSwitcher {
    fn default() -> Self {
        Self {
            default_scope: WorkspaceScope::All,
        }
    }
}

pub fn window_switcher_config() -> WindowSwitcher {
    cosmic_config::Config::new(
        APP_ID,
        WindowSwitcher::VERSION,
    )
    .ok()
    .and_then(|config| WindowSwitcher::get_entry(&config).ok())
    .unwrap_or_default()
}