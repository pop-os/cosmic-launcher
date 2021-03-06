// SPDX-License-Identifier: MPL-2.0-only

use crate::{
    application::{CosmicLauncherApplication, Event, TX},
    fl,
    search_result_object::SearchResultObject,
    search_result_row::SearchResultRow,
    utils,
};

use cascade::cascade;
use gdk4_x11::X11Display;
use gtk4::{
    gdk, gio, glib, glib::Object, prelude::*, subclass::prelude::*, Box, Entry, ListView,
    Orientation, SignalListItemFactory,
};
use libcosmic::x;
use std::path::Path;

mod imp {
    use super::*;
    use gtk4::{gio, glib};
    use gtk4::{Entry, ListView};
    use once_cell::sync::OnceCell;

    // Object holding the state
    #[derive(Default)]
    pub struct CosmicLauncherWindow {
        pub entry: OnceCell<Entry>,
        pub list_view: OnceCell<ListView>,
        pub model: OnceCell<gio::ListStore>,
        pub selection_model: OnceCell<gtk4::SingleSelection>,
        pub icon_theme: OnceCell<gtk4::IconTheme>,
    }

    // The central trait for subclassing a GObject
    #[glib::object_subclass]
    impl ObjectSubclass for CosmicLauncherWindow {
        // `NAME` needs to match `class` attribute of template
        const NAME: &'static str = "CosmicLauncherWindow";
        type Type = super::CosmicLauncherWindow;
        type ParentType = gtk4::ApplicationWindow;
    }

    // Trait shared by all GObjects
    impl ObjectImpl for CosmicLauncherWindow {}

    // Trait shared by all widgets
    impl WidgetImpl for CosmicLauncherWindow {}

    // Trait shared by all windows
    impl WindowImpl for CosmicLauncherWindow {}

    // Trait shared by all application
    impl ApplicationWindowImpl for CosmicLauncherWindow {}
}

glib::wrapper! {
    pub struct CosmicLauncherWindow(ObjectSubclass<imp::CosmicLauncherWindow>)
        @extends gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

const NUM_LAUNCHER_ITEMS: u8 = 9;

impl CosmicLauncherWindow {
    pub fn new(app: &CosmicLauncherApplication) -> Self {
        let self_: Self = Object::new(&[("application", app)]).expect("Failed to create `Window`.");
        let imp = imp::CosmicLauncherWindow::from_instance(&self_);

        cascade! {
            &self_;
            ..set_width_request(600);
            ..set_title(Some(&fl!("cosmic-launcher")));
            ..set_decorated(false);
            ..set_resizable(false);
            ..add_css_class("root_window");
            ..add_css_class("border-radius-small");
            ..add_css_class("padding-small");
        };

        let container = cascade! {
            Box::new(Orientation::Vertical, 0);
            ..add_css_class("background");
            ..add_css_class("border-radius-small");
        };
        self_.set_child(Some(&container));

        let entry = cascade! {
            Entry::new();
            ..set_margin_bottom(12);
            ..add_css_class("background-component");
            ..add_css_class("border-radius-medium");
            ..add_css_class("padding-medium");
        };
        container.append(&entry);

        let list_view = cascade! {
            ListView::default();
            ..set_orientation(Orientation::Vertical);
            ..set_single_click_activate(true);
            ..add_css_class("primary-container");
            ..add_css_class("border-radius-medium");
        };
        container.append(&list_view);

        imp.entry.set(entry).unwrap();
        imp.list_view.set(list_view).unwrap();

        let icon_theme = gtk4::IconTheme::for_display(&gdk::Display::default().unwrap());
        let data_dirs = utils::xdg_data_dirs();

        if utils::in_flatpak() {
            for mut p in data_dirs {
                if p.starts_with("/usr") {
                    let stripped_path = p.strip_prefix("/").unwrap_or(&p);
                    p = Path::new("/var/run/host").join(stripped_path);
                }
                let mut icons = p.clone();
                icons.push("icons");
                let mut pixmaps = p.clone();
                pixmaps.push("pixmaps");

                icon_theme.add_search_path(icons);
                icon_theme.add_search_path(pixmaps);
            }
        }
        // dbg!(icon_theme.search_path());
        // dbg!(icon_theme.icon_names());
        imp.icon_theme.set(icon_theme).unwrap();

        // Setup
        self_.setup_model();
        self_.setup_callbacks();
        self_.setup_factory();
        self_
    }

    pub fn model(&self) -> &gio::ListStore {
        // Get state
        let imp = imp::CosmicLauncherWindow::from_instance(self);
        imp.model.get().expect("Could not get model")
    }

    pub fn selected(&self) -> u32 {
        let imp = imp::CosmicLauncherWindow::from_instance(self);
        imp.selection_model.get().unwrap().selected()
    }

    fn activate_result(&self, position: u32) {
        let model = self.model();

        if position >= model.n_items() {
            dbg!("index out of range");
            return;
        }
        let obj = match model.item(position) {
            Some(obj) => obj.downcast::<SearchResultObject>().unwrap(),
            None => {
                dbg!(model.item(position));
                return;
            },
        };
        if let Some(search_result) = obj.data() {
            println!("activating... {}", position + 1);
            glib::MainContext::default().spawn_local(async move {
                if let Some(tx) = TX.get() {
                    let _ = tx.send(Event::Activate(search_result.id)).await;
                }
            });
        }
    }

    fn setup_model(&self) {
        // Get state and set model
        let imp = imp::CosmicLauncherWindow::from_instance(self);
        let model = gio::ListStore::new(SearchResultObject::static_type());

        let slice_model = gtk4::SliceListModel::new(Some(&model), 0, NUM_LAUNCHER_ITEMS.into());
        let selection_model = gtk4::SingleSelection::builder()
            .model(&slice_model)
            .autoselect(true)
            .build();

        imp.model.set(model).expect("Could not set model");
        // Wrap model with selection and pass it to the list view
        imp.list_view
            .get()
            .unwrap()
            .set_model(Some(&selection_model));
        imp.selection_model.set(selection_model).expect("Could not set selection model");
    }

    fn setup_callbacks(&self) {
        // Get state
        let imp = imp::CosmicLauncherWindow::from_instance(self);
        let window = self.clone();
        let list_view = &imp.list_view;
        let entry = &imp.entry.get().unwrap();
        let lv = list_view.get().unwrap();
        for i in 1..10 {
            let action_launchi = gio::SimpleAction::new(&format!("launch{}", i), None);
            self.add_action(&action_launchi);
            action_launchi.connect_activate(glib::clone!(@weak window => move |_action, _parameter| {
                window.activate_result(i - 1);
            }));
        }

        lv.connect_activate(glib::clone!(@weak window => move |_list_view, i| {
            window.activate_result(i);
        }));

        entry.connect_activate(glib::clone!(@weak window => move |_| {
            window.activate_result(window.selected());
        }));

        entry.connect_changed(glib::clone!(@weak lv => move |search: &gtk4::Entry| {
            let search = search.text().to_string();
            dbg!(&search);
            glib::MainContext::default().spawn_local(async move {
                if let Some(tx) = TX.get() {
                    println!("searching...");
                    if let Err(e) = tx.send(Event::Search(search)).await {
                        println!("{}", e);
                    }
                }
            });
        }));

        entry.connect_realize(glib::clone!(@weak lv => move |search: &gtk4::Entry| {
            let search = search.text().to_string();

            glib::MainContext::default().spawn_local(async move {
                println!("searching...");
                if let Some(tx) = TX.get() {
                    if let Err(e) = tx.send(Event::Search(search)).await {
                        println!("{}", e);
                    }
                }
            });
        }));

        window.connect_realize(move |window| {
            let _ = std::panic::catch_unwind(|| {
                // XXX investigate panic in libcosmic
                // seems to be a race
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Some((display, surface)) = x::get_window_x11(window) {
                // ignore all x11 errors...
                let xdisplay = display.clone().downcast::<X11Display>().expect("Failed to downgrade X11 Display.");
                xdisplay.error_trap_push();
                unsafe {
                    x::change_property(
                        &display,
                        &surface,
                        "_NET_WM_WINDOW_TYPE",
                        x::PropMode::Replace,
                        &[x::Atom::new(&display, "_NET_WM_WINDOW_TYPE_DIALOG").unwrap()],
                    );
                }
                let resize = glib::clone!(@weak window => move || {
                    let height = window.height();
                    let width = window.width();

                    if let Some((display, _surface)) = x::get_window_x11(&window) {
                        let geom = display
                            .primary_monitor().geometry();
                        let monitor_x = geom.x();
                        let monitor_y = geom.y();
                        let monitor_width = geom.width();
                        let monitor_height = geom.height();
                        // dbg!(monitor_width);
                        // dbg!(monitor_height);
                        // dbg!(width);
                        // dbg!(height);
                        unsafe { x::set_position(&display, &surface,
                            (monitor_x + monitor_width / 2 - width / 2).clamp(0, monitor_x + monitor_width - 1),
                            (monitor_y + monitor_height / 2 - height / 2).clamp(0, monitor_y + monitor_height - 1))};
                    }
                });
                let s = window.surface();
                let resize_height = resize.clone();
                s.connect_height_notify(move |_s| {
                    glib::source::idle_add_local_once(resize_height.clone());
                });
                let resize_width = resize.clone();
                s.connect_width_notify(move |_s| {
                    glib::source::idle_add_local_once(resize_width.clone());
                });
                s.connect_scale_factor_notify(move |_s| {
                    glib::source::idle_add_local_once(resize.clone());
                });
            } else {
                println!("failed to get X11 window");
            }
            });
        });

        let action_quit = gio::SimpleAction::new("quit", None);
        // TODO clear state instead of closing
        action_quit.connect_activate(glib::clone!(@weak entry  => move |_, _| {
            entry.set_text("");
        }));
        self.add_action(&action_quit);

        // TODO clear the search state on fucus loss
        window.connect_is_active_notify(glib::clone!(@weak entry => move |win| {
            if !win.is_active() {
                entry.set_text("");
            }
        }));
    }

    fn setup_factory(&self) {
        let factory = SignalListItemFactory::new();
        factory.connect_setup(move |_, list_item| {
            let row = SearchResultRow::new();
            list_item.set_child(Some(&row))
        });
        let imp = imp::CosmicLauncherWindow::from_instance(self);
        let icon_theme = imp.icon_theme.get().unwrap();
        factory.connect_bind(glib::clone!(@weak icon_theme => move |_, list_item| {
            let application_object = list_item
                .item()
                .expect("The item has to exist.")
                .downcast::<SearchResultObject>()
                .expect("The item has to be an `SearchResultObject`");
            let row = list_item
                .child()
                .expect("The list item child needs to exist.")
                .downcast::<SearchResultRow>()
                .expect("The list item type needs to be `SearchResultRow`");
            if list_item.position() < 9 {
                row.set_shortcut(list_item.position() + 1);
            }

            row.set_search_result(application_object, icon_theme);
        }));
        // Set the factory of the list view
        let imp = imp::CosmicLauncherWindow::from_instance(self);
        imp.list_view.get().unwrap().set_factory(Some(&factory));
    }
}
