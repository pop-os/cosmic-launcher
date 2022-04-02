// SPDX-License-Identifier: GPL-3.0-only

mod application;
mod search_result_object;
mod search_result_row;
#[rustfmt::skip]
mod config;
mod utils;
mod window;

use tokio::{runtime::Runtime};
use gettextrs::{gettext, LocaleCategory};
use gtk4::{gio, glib};

use self::application::CosmicLauncherApplication;
use self::config::{GETTEXT_PACKAGE, LOCALEDIR, RESOURCES_FILE};

fn main() {
    // Initialize logger
    pretty_env_logger::init();

    // Prepare i18n
    gettextrs::setlocale(LocaleCategory::LcAll, "");
    gettextrs::bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    gettextrs::textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    glib::set_application_name(&gettext("Cosmic Launcher"));

    let res = gio::Resource::load(RESOURCES_FILE).expect("Could not load gresource file");
    gio::resources_register(&res);
    let rt = Runtime::new().unwrap();

    let app = CosmicLauncherApplication::new(rt);
    app.run();
}
