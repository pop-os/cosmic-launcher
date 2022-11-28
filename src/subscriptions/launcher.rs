use cosmic::iced::futures::{channel::mpsc, StreamExt};
use futures::Stream;
use pop_launcher::{Request, Response};
use std::{hash::Hash, pin::Pin};

#[derive(Debug, Clone)]
pub enum LauncherRequest {
    Search(String),
    Activate(u32),
}

#[derive(Debug, Clone)]
pub enum LauncherEvent {
    Started(mpsc::Sender<LauncherRequest>),
    Response(pop_launcher::Response),
    Error(String),
}

pub fn launcher<I: 'static + Hash + Copy + Send + Sync>(
    id: I,
) -> cosmic::iced::Subscription<(I, LauncherEvent)> {
    use cosmic::iced::subscription;

    subscription::unfold(id, State::Ready, move |state| _launcher(id, state))
}

async fn _launcher<I: Copy>(id: I, state: State) -> (Option<(I, LauncherEvent)>, State) {
    match state {
        State::Ready => {
            if let Ok(launcher_ipc) = LauncherIpc::new() {
                (
                    Some((id, LauncherEvent::Started(launcher_ipc.get_sender()))),
                    State::Waiting(launcher_ipc),
                )
            } else {
                (
                    Some((
                        id,
                        LauncherEvent::Error("Failed to start the ipc client".to_string()),
                    )),
                    State::Error,
                )
            }
        }
        State::Waiting(mut rx) => {
            if let Some(response) = rx.results().await {
                (
                    Some((id, LauncherEvent::Response(response))),
                    State::Waiting(rx),
                )
            } else {
                (
                    Some((
                        id,
                        LauncherEvent::Error("channel for ipc client was closed".to_string()),
                    )),
                    State::Error,
                )
            }
        }
        State::Error => cosmic::iced::futures::future::pending().await,
    }
}

pub enum State {
    Ready,
    Waiting(LauncherIpc),
    Error,
}

pub struct LauncherIpc {
    ipc_rx: Pin<Box<dyn Stream<Item = Response> + Send>>,
    tx: mpsc::Sender<LauncherRequest>,
}

impl LauncherIpc {
    pub fn new() -> anyhow::Result<Self> {
        let (mut ipc_tx, ipc_rx) = pop_launcher_service::IpcClient::new()?;
        let (tx, mut rx) = mpsc::channel(100);
        tokio::spawn(async move {
            while let Some(req) = rx.next().await {
                match req {
                    LauncherRequest::Search(s) => {
                        let _ = ipc_tx.send(Request::Search(s)).await;
                    }
                    LauncherRequest::Activate(i) => {
                        let _ = ipc_tx.send(Request::Activate(i)).await;
                    }
                }
            }
        });
        Ok(Self {
            ipc_rx: Box::pin(ipc_rx),
            tx,
        })
    }

    pub fn get_sender(&self) -> mpsc::Sender<LauncherRequest> {
        self.tx.clone()
    }

    pub async fn results(&mut self) -> Option<Response> {
        self.ipc_rx.next().await
    }
}
