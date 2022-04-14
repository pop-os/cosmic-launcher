// SPDX-License-Identifier: MPL-2.0-only

mod application;
mod config;
mod desktop_entry_data;
mod search_result_object;
mod search_result_row;

mod utils;
mod window;
use gtk4::{gio, glib};
use tokio::runtime::Runtime;

use self::config::RESOURCES_FILE;

use self::application::CosmicLauncherApplication;

fn main() {
    // Initialize logger
    pretty_env_logger::init();

    glib::set_application_name("Cosmic Launcher");

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);
    let rt = Runtime::new().unwrap();

    let app = CosmicLauncherApplication::new(rt);
    app.run();
}
