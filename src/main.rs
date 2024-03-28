mod components;
#[rustfmt::skip]
mod config;
mod app;
mod localize;
mod subscriptions;
use tracing::info;

use localize::localize;

use crate::config::VERSION;

fn main() -> cosmic::iced::Result {
    // Initialize logger
    #[cfg(feature = "console")]
    if std::env::var("TOKIO_CONSOLE").as_deref() == Ok("1") {
        std::env::set_var("RUST_LOG", "trace");
        console_subscriber::init();
    }
    pretty_env_logger::init();

    info!(
        "cosmic-launcher ({})",
        <app::CosmicLauncher as cosmic::Application>::APP_ID
    );
    info!("Version: {} ({})", VERSION, config::profile());

    // Prepare i18n
    localize();

    app::run()
}
