// Copyright 2026 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Settings of the COSMIC launcher, shared with COSMIC Settings.

use cosmic_config::{CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

/// cosmic-config id the settings are stored under.
pub const APP_ID: &str = "com.system76.CosmicLauncher";

/// Which workspaces the window switcher (Alt+Tab) lists windows from.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum WindowSwitcherScope {
    /// Every window on every workspace.
    #[default]
    AllWorkspaces,
    /// Windows on the workspace currently shown on each display.
    ActiveWorkspaces,
    /// Windows on the workspace shown on the display the switcher appears on
    /// (the compositor's active display, which follows the pointer).
    FocusedOutputWorkspace,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize, Serialize, CosmicConfigEntry)]
#[version = 1]
pub struct CosmicLauncherConfig {
    /// Which workspaces the window switcher lists windows from.
    pub window_switcher_scope: WindowSwitcherScope,
}
