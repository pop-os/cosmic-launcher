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
    // Always software rendering unless `COSMIC_LAUNCHER_WGPU=1` then try wgpu first
    #[cfg(feature = "wgpu")]
    {
        let backend = if std::env::var("COSMIC_LAUNCHER_WGPU").as_deref() == Ok("1") {
            "wgpu,tiny-skia"
        } else {
            "tiny-skia"
        };
        // SAFETY: no other threads exist yet.
        unsafe { std::env::set_var("ICED_BACKEND", backend) };
    }

    init_logging();

    info!(
        "cosmic-launcher ({})",
        <app::CosmicLauncher as cosmic::Application>::APP_ID
    );
    info!("Version: {} ({})", VERSION, config::profile());

    // Prepare i18n
    localize();

    app::run()
}

fn init_logging() {
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;
    use tracing_subscriber::{EnvFilter, fmt};

    // Initialize logger
    #[cfg(feature = "console")]
    if std::env::var("TOKIO_CONSOLE").as_deref() == Ok("1") {
        unsafe {
            std::env::set_var("RUST_LOG", "trace");
        }
        console_subscriber::init();
    }

    let filter_layer = EnvFilter::try_from_default_env().unwrap_or(if cfg!(debug_assertions) {
        EnvFilter::new(format!("warn,{}=debug", env!("CARGO_CRATE_NAME")))
    } else {
        EnvFilter::new("warn")
    });

    let fmt_layer = fmt::layer().with_target(false);

    if let Ok(journal_layer) = tracing_journald::layer() {
        tracing_subscriber::registry()
            .with(journal_layer)
            .with(filter_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(filter_layer)
            .init();
    }
}
