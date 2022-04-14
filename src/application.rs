// SPDX-License-Identifier: MPL-2.0-only

use crate::search_result_object::SearchResultObject;
use crate::utils::BoxedSearchResult;
use gtk4::{gdk::Display, gio, glib, prelude::*, subclass::prelude::*, CssProvider, StyleContext};
use log::{debug, info};
use once_cell::sync::OnceCell;
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_stream::StreamExt;

use crate::{utils, window::CosmicLauncherWindow};

pub const NUM_LAUNCHER_ITEMS: u8 = 10;
pub static TX: OnceCell<mpsc::Sender<Event>> = OnceCell::new();
pub enum Event {
    Response(pop_launcher::Response),
    Search(String),
    Activate(u32),
}

pub enum LauncherIpcEvent {
    Response(pop_launcher::Response),
    Request(pop_launcher::Request),
}

mod imp {
    use std::{fs, path::Path};

    use crate::desktop_entry_data::DesktopEntryData;

    use super::*;
    use glib::WeakRef;
    use once_cell::sync::OnceCell;

    #[derive(Debug, Default)]
    pub struct CosmicLauncherApplication {
        pub window: OnceCell<WeakRef<CosmicLauncherWindow>>,
        pub rt: OnceCell<Runtime>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CosmicLauncherApplication {
        const NAME: &'static str = "CosmicLauncherApplication";
        type Type = super::CosmicLauncherApplication;
        type ParentType = gtk4::Application;
    }

    impl ObjectImpl for CosmicLauncherApplication {}

    impl ApplicationImpl for CosmicLauncherApplication {
        fn activate(&self, app: &Self::Type) {
            debug!("GtkApplication<CosmicLauncherApplication>::activate");
            self.parent_activate(app);

            if let Some(window) = self.window.get() {
                let window = window.upgrade().unwrap();
                window.present();
                return;
            }

            let window = CosmicLauncherWindow::new(app);
            self.window
                .set(window.downgrade())
                .expect("Window already set.");

            let (tx, mut rx) = mpsc::channel(100);
            let (launcher_tx, launcher_rx) = mpsc::channel(100);
            let rt = self.rt.get().unwrap();
            rt.spawn(spawn_launcher(tx.clone(), launcher_rx));
            if TX.set(tx).is_err() {
                println!("failed to set global Sender. Exiting");
                std::process::exit(1);
            };

            let window = CosmicLauncherWindow::new(app);
            window.show();

            let app_clone = app.clone();
            glib::MainContext::default().spawn_local(async move {
                while let Some(event) = rx.recv().await {
                    match event {
                        Event::Search(search) => {
                            let _ = launcher_tx
                                .send(pop_launcher::Request::Search(search))
                                .await;
                        }
                        Event::Activate(index) => {
                            let _ = launcher_tx
                                .send(pop_launcher::Request::Activate(index))
                                .await;
                        }
                        Event::Response(event) => {
                            if let pop_launcher::Response::Update(results) = event {
                                let model = window.model();
                                let model_len = model.n_items();
                                let new_results: Vec<glib::Object> = results
                                    // [0..std::cmp::min(results.len(), NUM_LAUNCHER_ITEMS.into())]
                                    .into_iter()
                                    .map(|result| {
                                        SearchResultObject::new(&BoxedSearchResult(Some(result)))
                                            .upcast()
                                    })
                                    .collect();
                                model.splice(0, model_len, &new_results[..]);
                            } else if let pop_launcher::Response::DesktopEntry {
                                mut path,
                                gpu_preference: _gpu_preference, // TODO use GPU preference when launching app
                            } = event
                            {
                                if utils::in_flatpak() {
                                    if path.starts_with("/usr") {
                                        let stripped_path = path.strip_prefix("/").unwrap_or(&path);
                                        path = Path::new("/var/run/host").join(stripped_path);
                                    }
                                }
                                if let Ok(bytes) = fs::read_to_string(&path) {
                                    if let Ok(de) = freedesktop_desktop_entry::DesktopEntry::decode(
                                        &path, &bytes,
                                    ) {
                                        let name: String = de.name(None).unwrap_or_default().into();
                                        if name.eq("".into()) || de.no_display() {
                                            continue;
                                        };
                                        // dbg!(de.appid);
                                        let app_info = DesktopEntryData::new();
                                        app_info.set_data(
                                            path.file_stem()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                                .into(),
                                            path.clone(),
                                            name,
                                            de.icon().map(|s| String::from(s)),
                                            de.categories().unwrap_or_default().into(),
                                        );
                                        app_info
                                            .launch()
                                            .expect("failed to launch the application.");
                                        app_clone.quit();
                                    }
                                }
                            }
                        }
                    }
                }
            });
        }

        fn startup(&self, app: &Self::Type) {
            debug!("GtkApplication<CosmicLauncherApplication>::startup");
            self.parent_startup(app);

            // Set icons for shell
            gtk4::Window::set_default_icon_name(crate::APP_ID);

            setup_shortcuts(app);
            load_css();
        }
    }

    impl GtkApplicationImpl for CosmicLauncherApplication {}
}

glib::wrapper! {
    pub struct CosmicLauncherApplication(ObjectSubclass<imp::CosmicLauncherApplication>)
        @extends gio::Application, gtk4::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl CosmicLauncherApplication {
    pub fn new(rt: Runtime) -> Self {
        let self_: Self = glib::Object::new(&[
            ("application-id", &Some(crate::APP_ID)),
            ("flags", &gio::ApplicationFlags::empty()),
            ("resource-base-path", &Some("/com/System76/CosmicLauncher/")),
        ])
        .expect("Application initialization failed...");
        self_.imp().rt.set(rt).unwrap();
        self_
    }

    pub fn run(&self) {
        ApplicationExtManual::run(self);
    }
}

async fn spawn_launcher(tx: mpsc::Sender<Event>, mut rx: mpsc::Receiver<pop_launcher::Request>) {
    // TODO cleanup

    if utils::in_flatpak() {
        let (mut launcher, responses) = pop_launcher_service::IpcClient::new_flatpak()
            .expect("failed to connect to launcher service");
        let launcher_stream = Box::pin(async_stream::stream! {
            while let Some(e) = rx.recv().await {
                yield LauncherIpcEvent::Request(e);
            }
        });

        let responses = Box::pin(responses.map(|e| LauncherIpcEvent::Response(e)));
        let mut rx = launcher_stream.merge(responses);
        while let Some(event) = rx.next().await {
            match event {
                LauncherIpcEvent::Response(e) => {
                    let _ = tx.send(Event::Response(e)).await;
                }
                LauncherIpcEvent::Request(e) => {
                    let _ = launcher.send(e).await;
                }
            }
        }
    } else {
        let (mut launcher, responses) =
            pop_launcher_service::IpcClient::new().expect("failed to connect to launcher service");
        let launcher_stream = Box::pin(async_stream::stream! {
            while let Some(e) = rx.recv().await {
                yield LauncherIpcEvent::Request(e);
            }
        });

        let responses = Box::pin(responses.map(|e| LauncherIpcEvent::Response(e)));
        let mut rx = launcher_stream.merge(responses);
        while let Some(event) = rx.next().await {
            match event {
                LauncherIpcEvent::Response(e) => {
                    let _ = tx.send(Event::Response(e)).await;
                }
                LauncherIpcEvent::Request(e) => {
                    let _ = launcher.send(e).await;
                }
            }
        }
    };
}

fn setup_shortcuts(app: &CosmicLauncherApplication) {
    //quit shortcut
    app.set_accels_for_action("win.quit", &["<primary>W", "Escape"]);
    //launch shortcuts
    for i in 1..NUM_LAUNCHER_ITEMS {
        app.set_accels_for_action(&format!("win.launch{}", i), &[&format!("<primary>{}", i)]);
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_resource("/com/System76/CosmicLauncher/style.css");
    // Add the provider to the default screen
    StyleContext::add_provider_for_display(
        &Display::default().expect("Error initializing GTK CSS provider."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let theme_provider = CssProvider::new();
    // Add the provider to the default screen
    StyleContext::add_provider_for_display(
        &Display::default().expect("Error initializing GTK CSS provider."),
        &theme_provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Load the css file and add it to the provider
    glib::MainContext::default().spawn_local(async move {
        if let Err(e) = cosmic_theme::load_cosmic_gtk4_theme(theme_provider).await {
            eprintln!("{}", e);
        }
    });
}
