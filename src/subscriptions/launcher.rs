use cosmic::{iced::futures::StreamExt, iced_runtime::futures::MaybeSend};
use futures::{SinkExt, Stream};
use pop_launcher_service::IpcClient;
use std::hash::Hash;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
pub enum Request {
    Search(String),
    Activate(u32),
    Context(u32),
    ActivateContext(u32, u32),
    Close,
}

#[derive(Debug, Clone)]
pub enum Event {
    Started(mpsc::Sender<Request>),
    Response(pop_launcher::Response),
}

pub fn subscription<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
) -> cosmic::iced::Subscription<Event> {
    use cosmic::iced::subscription;

    subscription::channel(id, 1, |mut output| async move {
        loop {
            tracing::info!("starting pop-launcher service");
            let mut responses = std::pin::pin!(service());
            while let Some(message) = responses.next().await {
                let _res = output.send(message).await;
            }
        }
    })
}

/// Initializes pop-launcher if it is not running, and returns a handle to its client.
async fn client_request<'a>(
    tx: &mpsc::Sender<Event>,
    client: &'a mut Option<(IpcClient, oneshot::Sender<()>)>,
) -> &'a mut Option<(IpcClient, oneshot::Sender<()>)> {
    if client.is_none() {
        *client = match pop_launcher_service::IpcClient::new() {
            Ok((mut new_client, responses)) => {
                let tx = tx.clone();

                let (kill_tx, kill_rx) = tokio::sync::oneshot::channel();

                let _res = tokio::task::Builder::new()
                    .name("pop-launcher listener")
                    .spawn(async {
                        tracing::info!("starting pop-launcher instance");
                        let listener = Box::pin(async move {
                            let mut responses = std::pin::pin!(responses);
                            while let Some(response) = responses.next().await {
                                let _res = tx.send(Event::Response(response)).await;
                            }
                        });

                        let killswitch = Box::pin(async move {
                            let _res = kill_rx.await;
                        });

                        futures::future::select(listener, killswitch).await;
                    });

                let _res = new_client
                    .send(pop_launcher::Request::Search(String::new()))
                    .await;

                Some((new_client, kill_tx))
            }
            Err(why) => {
                tracing::error!("pop-launcher failed to start: {}", why);
                None
            }
        }
    };

    client
}

pub fn service() -> impl Stream<Item = Event> + MaybeSend {
    let (requests_tx, mut requests_rx) = mpsc::channel(4);
    let (responses_tx, mut responses_rx) = mpsc::channel(4);

    let _res = tokio::task::Builder::new()
        .name("pop-launcher forwarder")
        .spawn(async move {
            let _res = responses_tx.send(Event::Started(requests_tx.clone())).await;

            let client = &mut None;

            while let Some(request) = requests_rx.recv().await {
                match request {
                    Request::Search(s) => {
                        if let Some((client, _)) = client_request(&responses_tx, client).await {
                            let _res = client.send(pop_launcher::Request::Search(s)).await;
                        }
                    }
                    Request::Activate(i) => {
                        if let Some((client, _)) = client_request(&responses_tx, client).await {
                            let _res = client.send(pop_launcher::Request::Activate(i)).await;
                        }
                    }
                    Request::Context(i) => {
                        if let Some((client, _)) = client_request(&responses_tx, client).await {
                            let _res = client.send(pop_launcher::Request::Context(i)).await;
                        }
                    }
                    Request::ActivateContext(id, context) => {
                        if let Some((client, _)) = client_request(&responses_tx, client).await {
                            let _res = client.send(pop_launcher::Request::ActivateContext { id, context }).await;
                        }
                    }
                    Request::Close => {
                        if let Some((mut client, kill)) = client.take() {
                            tracing::info!("closing pop-launcher instance");
                            let _res = kill.send(());
                            let _res = client.child.kill().await;
                            let _res = client.child.wait().await;
                        }
                    }
                }
            }
        });

    async_stream::stream! {
        while let Some(message) = responses_rx.recv().await {
            yield message;
        }
    }
}
