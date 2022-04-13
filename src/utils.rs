// SPDX-License-Identifier: MPL-2.0-only

use gtk4::{gio, glib};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone, Debug, Default, glib::Boxed)]
#[boxed_type(name = "BoxedSearchResult")]
pub struct BoxedSearchResult(pub Option<pop_launcher::SearchResult>);

pub fn icon_source(
    image: &Rc<RefCell<gtk4::Image>>,
    source: &Option<pop_launcher::IconSource>,
    icon_theme: &gtk4::IconTheme,
) {
    if !in_flatpak() {
        match source {
            Some(pop_launcher::IconSource::Name(name)) => {
                image.borrow().set_from_icon_name(Some(name));
            }
            Some(pop_launcher::IconSource::Mime(content_type)) => {
                image
                    .borrow()
                    .set_from_gicon(&gio::content_type_get_icon(content_type));
            }
            _ => {
                image.borrow().set_from_icon_name(None);
            }
        }
    }
    let icon_name = match source {
        Some(pop_launcher::IconSource::Name(name)) => name,

        Some(pop_launcher::IconSource::Mime(content_type)) => content_type,
        _ => "",
    };

    let mut p = PathBuf::from(&icon_name);
    if p.has_root() {
        if p.starts_with("/usr") {
            let stripped_path = p.strip_prefix("/").unwrap_or(&p);
            p = Path::new("/var/run/host").join(stripped_path);
        }
        image.borrow().set_from_file(Some(p));
    } else {
        let icon_size = icon_theme
            .icon_sizes(&icon_name)
            .into_iter()
            .max()
            .unwrap_or(1);
        let icon = icon_theme.lookup_icon(
            &icon_name,
            &[],
            icon_size,
            1,
            gtk4::TextDirection::Ltr,
            gtk4::IconLookupFlags::PRELOAD,
        );
        image.borrow().set_paintable(Some(&icon));
    };
}

pub fn in_flatpak() -> bool {
    std::env::var("FLATPAK_ID").is_ok()
}

pub fn xdg_data_dirs() -> Vec<PathBuf> {
    if in_flatpak() {
        std::str::from_utf8(
            &std::process::Command::new("flatpak-spawn")
                .args(["--host", "printenv", "XDG_DATA_DIRS"])
                .output()
                .unwrap()
                .stdout[..],
        )
        .unwrap_or_default()
        .trim()
        .split(":")
        .map(|p| PathBuf::from(p))
        .collect()
    } else {
        let xdg_base = xdg::BaseDirectories::new().expect("could not access XDG Base directory");
        let data_dirs = xdg_base.get_data_dirs();
        data_dirs
    }
}
