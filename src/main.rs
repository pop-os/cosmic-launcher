// SPDX-License-Identifier: GPL-3.0-only
use crate::utils::BoxedSearchResult;
use gtk4::{
    gdk::Display, gio::DesktopAppInfo, glib, prelude::*, Application, CssProvider, StyleContext,
};
use once_cell::sync::OnceCell;
use tokio::{runtime::Runtime, sync::mpsc};
use tokio_stream::StreamExt;

use self::search_result_object::SearchResultObject;
use self::window::Window;

mod search_result_object;
mod search_result_row;
mod utils;
mod window;

const NUM_LAUNCHER_ITEMS: u8 = 10;
static TX: OnceCell<mpsc::Sender<Event>> = OnceCell::new();

pub enum Event {
    Response(pop_launcher::Response),
    Search(String),
    Activate(u32),
}

pub enum LauncherIpcEvent {
    Response(pop_launcher::Response),
    Request(pop_launcher::Request),
}

async fn spawn_launcher(tx: mpsc::Sender<Event>, mut rx: mpsc::Receiver<pop_launcher::Request>) {
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
}

fn setup_shortcuts(app: &Application) {
    //quit shortcut
    app.set_accels_for_action("win.quit", &["<primary>W", "Escape"]);
    //launch shortcuts
    for i in 1..NUM_LAUNCHER_ITEMS {
        app.set_accels_for_action(&format!("win.launch{}", i), &[&format!("<primary>{}", i)]);
    }
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_data(include_bytes!("style.css"));

    // Add the provider to the default screen
    StyleContext::add_provider_for_display(
        &Display::default().expect("Error initializing GTK CSS provider."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    // Load the css file and add it to the provider
    glib::MainContext::default().spawn_local(async move {
        if let Err(e) = cosmic_theme::load_cosmic_gtk_theme().await {
            eprintln!("{}", e);
        }
    });
}

fn main() {
    let app = gtk4::Application::builder()
        .application_id("com.cosmic.Launcher")
        .build();

    app.connect_startup(|app| {
        setup_shortcuts(app);
        load_css();
    });
    let rt = Runtime::new().unwrap();
    app.connect_activate(move |app| {
        let (tx, mut rx) = mpsc::channel(100);
        let (launcher_tx, launcher_rx) = mpsc::channel(100);

        rt.spawn(spawn_launcher(tx.clone(), launcher_rx));
        if TX.set(tx).is_err() {
            println!("failed to set global Sender. Exiting");
            std::process::exit(1);
        };

        let window = Window::new(app);
        window.show();

        glib::MainContext::default().spawn_local(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    Event::Search(search) => {
                        let _ = launcher_tx.send(pop_launcher::Request::Search(search)).await;
                    }
                    Event::Activate(index) => {
                        let _ = launcher_tx.send(pop_launcher::Request::Activate(index)).await;
                    }

                    Event::Response(event) => {
                        if let pop_launcher::Response::Update(results) = event {
                            let model = window.model();
                            let model_len = model.n_items();
                            let new_results: Vec<glib::Object> = results
                                // [0..std::cmp::min(results.len(), NUM_LAUNCHER_ITEMS.into())]
                                .into_iter()
                                .map(|result| SearchResultObject::new(&BoxedSearchResult(Some(result))).upcast())
                                .collect();
                            model.splice(0, model_len, &new_results[..]);
                        } else if let pop_launcher::Response::DesktopEntry {
                            path,
                            gpu_preference: _gpu_preference, // TODO use GPU preference when launching app
                        } = event
                        {
                            let app_info =
                                DesktopAppInfo::new(&path.file_name().expect("desktop entry path needs to be a valid filename").to_string_lossy())
                                    .expect("failed to create a Desktop App info for launching the application.");
                            app_info
                                .launch(&[], Some(&window.display().app_launch_context())).expect("failed to launch the application.");
                        }
                    }
                }
            }
        });
    });

    app.run();
}
