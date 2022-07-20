// SPDX-License-Identifier: MPL-2.0-only

use crate::utils;
use anyhow::Result;
use gtk4::glib;
use gtk4::subclass::prelude::*;
use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::rc::Rc;
mod imp {

    use super::*;

    #[derive(Debug, Default)]
    pub struct DesktopEntryData {
        pub appid: Rc<RefCell<String>>,
        pub path: Rc<RefCell<PathBuf>>,
        pub name: Rc<RefCell<String>>,
        pub categories: Rc<RefCell<String>>,
        pub icon: Rc<RefCell<Option<String>>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DesktopEntryData {
        const NAME: &'static str = "DesktopEntryData";
        type Type = super::DesktopEntryData;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for DesktopEntryData {}
}

glib::wrapper! {
    pub struct DesktopEntryData(ObjectSubclass<imp::DesktopEntryData>);
}

impl DesktopEntryData {
    pub fn new() -> Self {
        glib::Object::new(&[]).unwrap()
    }

    pub fn set_data(
        &self,
        appid: String,
        path: PathBuf,
        name: String,
        icon: Option<String>,
        categories: String,
    ) {
        let imp = imp::DesktopEntryData::from_instance(self);
        imp.name.replace(name);
        imp.path.replace(path);
        imp.appid.replace(appid);
        imp.icon.replace(icon);
        imp.categories.replace(categories);
    }

    pub fn name(&self) -> String {
        imp::DesktopEntryData::from_instance(self)
            .name
            .borrow()
            .clone()
    }

    pub fn path(&self) -> PathBuf {
        imp::DesktopEntryData::from_instance(self)
            .path
            .borrow()
            .clone()
    }

    pub fn categories(&self) -> String {
        imp::DesktopEntryData::from_instance(self)
            .categories
            .borrow()
            .clone()
    }

    pub fn appid(&self) -> String {
        imp::DesktopEntryData::from_instance(self)
            .appid
            .borrow()
            .clone()
    }

    pub fn icon(&self) -> Option<String> {
        imp::DesktopEntryData::from_instance(self)
            .icon
            .borrow()
            .clone()
    }

    pub fn launch(&self) -> Result<Child> {
        println!(
            "starting {}",
            imp::DesktopEntryData::from_instance(self)
                .appid
                .borrow()
                .clone()
        );
        if utils::in_flatpak() {
            Command::new("flatpak-spawn")
                .arg("--host")
                .arg("gtk-launch")
                .arg(
                    imp::DesktopEntryData::from_instance(self)
                        .appid
                        .borrow()
                        .clone(),
                )
                .spawn()
                .map_err(anyhow::Error::msg)
        } else {
            let wayland_display = if let Ok(display) = std::env::var("WAYLAND_DISPLAY") {
                Some(("WAYLAND_DISPLAY", display))
            } else {
                None
            };
            Command::new("gtk-launch")
                .arg(
                    imp::DesktopEntryData::from_instance(self)
                        .appid
                        .borrow()
                        .clone(),
                )
                .envs(wayland_display)
                .spawn()
                .map_err(anyhow::Error::msg)
        }
    }
}
