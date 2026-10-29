use cosmic::iced::futures::StreamExt;
use cosmic::iced::runtime::futures::MaybeSend;
use cosmic::iced::{Subscription, stream};
use cosmic_launcher_config::Config;
use futures::{SinkExt, Stream};
use pop_launcher_service::{Args, IpcClient};
use std::hash::Hash;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub enum Request {
    Search(String),
    Activate(u32),
    Context(u32),
    Complete(u32),
    ActivateContext(u32, u32),
    Close,
    ServiceIsClosed,
    Config(Config),
}

#[derive(Debug, Clone)]
pub enum Event {
    Started(mpsc::Sender<Request>),
    Response(pop_launcher::Response),
    ServiceIsClosed,
    Config(Config),
}

pub fn subscription<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
    config: Config,
) -> cosmic::iced::Subscription<Event> {
    Subscription::run_with((id, config), |(_, config)| {
        let config = config.clone();
        stream::channel(
            1,
            |mut output: futures::channel::mpsc::Sender<Event>| async move {
                loop {
                    tracing::info!("starting pop-launcher service");
                    let mut responses = std::pin::pin!(service(config.clone()));
                    while let Some(message) = responses.next().await {
                        let _res = output.send(message).await;
                    }
                }
            },
        )
    })
}

/// Initializes pop-launcher if it is not running, and returns a handle to its client.
fn client_request<'a>(
    tx: &mpsc::Sender<Event>,
    client: &'a mut Option<(IpcClient, oneshot::Sender<()>)>,
    disabled_plugins: &[String],
) -> &'a mut Option<(IpcClient, oneshot::Sender<()>)> {
    if client.is_none() {
        *client = match pop_launcher_service::IpcClient::new_with_args(Args {
            max_files: 20,
            max_open: 99,
            max_search: 20,
            disable: disabled_plugins.to_vec(),
        }) {
            Ok((new_client, responses)) => {
                let tx = tx.clone();

                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();
                let listener = async {
                    tracing::info!("starting pop-launcher instance");
                    let listener = Box::pin(async move {
                        let mut responses = std::pin::pin!(responses);
                        while let Some(response) = responses.next().await {
                            _ = tx.send(Event::Response(response)).await;
                        }
                        _ = tx.send(Event::ServiceIsClosed).await;
                    });

                    let killswitch = Box::pin(async move {
                        let _res = kill_rx.await;
                    });

                    futures::future::select(listener, killswitch).await;
                };

                #[cfg(feature = "console")]
                let _res = tokio::task::Builder::new()
                    .name("pop-launcher listener")
                    .spawn(listener);

                #[cfg(not(feature = "console"))]
                let _res = tokio::task::spawn(listener);

                Some((new_client, kill_tx))
            }
            Err(why) => {
                tracing::error!("pop-launcher failed to start: {}", why);
                None
            }
        }
    }

    client
}

pub fn service(config: Config) -> impl Stream<Item = Event> + MaybeSend {
    let (requests_tx, mut requests_rx) = mpsc::channel(4);
    let (responses_tx, responses_rx) = mpsc::channel(4);

    let service_future = async move {
        let _res = responses_tx.send(Event::Started(requests_tx.clone())).await;

        let client = &mut None;

        // list of disabled plugins to pass to pop-launcher as arguments
        let mut disabled_plugins = config
            .plugins
            .into_iter()
            .filter_map(|plugin| plugin.disabled.then_some(plugin.name))
            .collect::<Vec<_>>();

        // send ListPlugin request once to update and save current plugins's state in pop-launcher
        if let Some((client, _)) = client_request(&responses_tx, client, &disabled_plugins) {
            let _res = client.send(pop_launcher::Request::ListPlugin).await;
        }

        while let Some(request) = requests_rx.recv().await {
            match request {
                Request::Search(s) => {
                    if let Some((client, _)) =
                        client_request(&responses_tx, client, &disabled_plugins)
                    {
                        let _res = client.send(pop_launcher::Request::Search(s)).await;
                    }
                }
                Request::Activate(i) => {
                    if let Some((client, _)) =
                        client_request(&responses_tx, client, &disabled_plugins)
                    {
                        let _res = client.send(pop_launcher::Request::Activate(i)).await;
                    }
                }
                Request::Context(i) => {
                    if let Some((client, _)) =
                        client_request(&responses_tx, client, &disabled_plugins)
                    {
                        let _res = client.send(pop_launcher::Request::Context(i)).await;
                    }
                }
                Request::ActivateContext(id, context) => {
                    if let Some((client, _)) =
                        client_request(&responses_tx, client, &disabled_plugins)
                    {
                        let _res = client
                            .send(pop_launcher::Request::ActivateContext { id, context })
                            .await;
                    }
                }
                Request::Close => {
                    if let Some((client, _)) =
                        client_request(&responses_tx, client, &disabled_plugins)
                    {
                        let _res = client.send(pop_launcher::Request::Close).await;
                    }
                }
                Request::Complete(id) => {
                    if let Some((client, _)) =
                        client_request(&responses_tx, client, &disabled_plugins)
                    {
                        let _res = client.send(pop_launcher::Request::Complete(id)).await;
                    }
                }
                Request::ServiceIsClosed => {
                    *client = None;
                }
                // update disabled plugins from config subscription, also stop client to re-init with new config
                Request::Config(config) => {
                    disabled_plugins = config
                        .plugins
                        .into_iter()
                        .filter_map(|plugin| plugin.disabled.then_some(plugin.name))
                        .collect();
                    *client = None;
                }
            }
        }
    };

    #[cfg(feature = "console")]
    let _res = tokio::task::Builder::new()
        .name("pop-launcher forwarder")
        .spawn(service_future);

    #[cfg(not(feature = "console"))]
    let _res = tokio::task::spawn(service_future);

    tokio_stream::wrappers::ReceiverStream::new(responses_rx)
}
