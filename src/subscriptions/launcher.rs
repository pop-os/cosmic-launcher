use cosmic::{iced::futures::{channel::mpsc, StreamExt}, iced_runtime::futures::MaybeSend};
use futures::{Stream, SinkExt};
use pop_launcher::Request;
use pop_launcher_service::IpcClient;
use std::hash::Hash;

#[derive(Debug, Clone)]
pub enum LauncherRequest {
    Search(String),
    Activate(u32),
    Close,
}

#[derive(Debug, Clone)]
pub enum LauncherEvent {
    Started(mpsc::Sender<LauncherRequest>),
    Response(pop_launcher::Response),
}

pub fn launcher<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
) -> cosmic::iced::Subscription<LauncherEvent> {
    use cosmic::iced::subscription;

    subscription::channel(id, 1, |mut output| async move {
        loop {
            log::info!("starting pop-launcher service");
            let mut responses = service();
            while let Some(message) = responses.next().await {
                let _res = output.send(message).await;
            }
        }
    })
}

/// Initializes pop-launcher if it is not running, and returns a handle to its client.
fn client_request<'a>(tx: &mpsc::Sender<LauncherEvent>, client: &'a mut Option<IpcClient>) -> &'a mut Option<IpcClient> {
    if client.is_none() {
        *client = match pop_launcher_service::IpcClient::new() {
            Ok((new_client, responses)) => {
                let mut tx = tx.clone();

                tokio::spawn(async move {
                    let mut responses = std::pin::pin!(responses);
                    while let Some(response) = responses.next().await {
                        let _res = tx.send(LauncherEvent::Response(response)).await;
                    }
                });

                Some(new_client)
            },
            Err(why) => {
                log::error!("pop-launcher failed to start: {}", why);
                None
            }
        }
    };

    client
}

pub fn service() -> impl Stream<Item = LauncherEvent> + MaybeSend {
    let (requests_tx, mut requests_rx) = mpsc::channel(4);
    let (mut responses_tx, responses_rx) = mpsc::channel(4);

    tokio::spawn(async move {
        let _res = responses_tx.send(LauncherEvent::Started(requests_tx.clone())).await;

        let client = &mut None;

        while let Some(request) = requests_rx.next().await {
            match request {
                LauncherRequest::Search(s) => {
                    if let Some(client) = client_request(&responses_tx, client) {
                        let _res = client.send(Request::Search(s)).await;
                    }
                }
                LauncherRequest::Activate(i) => {
                    if let Some(client) = client_request(&responses_tx, client) {
                        let _res = client.send(Request::Activate(i)).await;
                    }
                }
                LauncherRequest::Close => {
                    if let Some(mut client) = client.take() {
                        log::info!("closing pop-launcher service");
                        let _res = client.child.kill().await;
                        let _res = client.child.wait().await;
                    }
                }
            }
        }
    });

    responses_rx
}
