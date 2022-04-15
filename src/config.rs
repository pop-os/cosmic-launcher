pub const APP_ID: &str = "com.System76.CosmicLauncher";
#[cfg(feature = "dev")]
pub const RESOURCES_FILE: &str = "target/compiled.gresource";
#[cfg(not(feature = "dev"))]
pub const RESOURCES_FILE: &str = "/usr/share/com.System76.CosmicLauncher/compiled.gresource";
pub const VERSION: &str = "0.0.1";
