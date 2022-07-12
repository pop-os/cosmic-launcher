// SPDX-License-Identifier: MPL-2.0-only

mod application;
mod config;
mod desktop_entry_data;
mod search_result_object;
mod search_result_row;

mod localize;
mod utils;
mod window;
use gtk4::{gio, glib};
use tokio::runtime::Runtime;

use self::application::CosmicLauncherApplication;

pub fn localize() {
    let localizer = crate::localize::localizer();
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    if let Err(error) = localizer.select(&requested_languages) {
        eprintln!(
            "Error while loading language for pop-desktop-widget {}",
            error
        );
    }
}

fn main() {
    // Initialize logger
    pretty_env_logger::init();

    localize();
    gio::resources_register_include!("compiled.gresource").unwrap();
    let rt = Runtime::new().unwrap();

    let app = CosmicLauncherApplication::new(rt);
    app.run();
}
