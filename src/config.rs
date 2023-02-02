pub const APP_ID: &str = "com.system76.CosmicLauncher";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn profile() -> &'static str {
    std::env!("OUT_DIR")
        .split(std::path::MAIN_SEPARATOR)
        .nth_back(3)
        .unwrap_or("unknown")
}
