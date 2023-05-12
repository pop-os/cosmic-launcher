use cosmic::iced::subscription;
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    StreamExt,
};
use std::{fmt::Debug, hash::Hash};
use zbus::{dbus_interface, Connection, ConnectionBuilder};

// todo refactor to use subscription channel
pub fn dbus_toggle<I: 'static + Hash + Copy + Send + Sync + Debug>(
    id: I,
) -> cosmic::iced::Subscription<Option<(I, LauncherDbusEvent)>> {
    subscription::unfold(id, State::Ready, move |state| start_listening(id, state))
}

#[derive(Debug)]
pub enum State {
    Ready,
    Waiting(Connection, UnboundedReceiver<LauncherDbusEvent>),
    Finished,
}

async fn start_listening<I: Copy>(id: I, state: State) -> (Option<(I, LauncherDbusEvent)>, State) {
    match state {
        State::Ready => {
            let (tx, rx) = unbounded();
            if let Some(conn) = ConnectionBuilder::session()
                .ok()
                .and_then(|conn| conn.name("com.system76.CosmicLauncher").ok())
                .and_then(|conn| {
                    conn.serve_at("/com/system76/CosmicLauncher", CosmicLauncherServer { tx })
                        .ok()
                })
                .map(|conn| conn.build())
            {
                if let Ok(conn) = conn.await {
                    return (None, State::Waiting(conn, rx));
                }
            }
            return (None, State::Finished);
        }
        State::Waiting(conn, mut rx) => {
            if let Some(LauncherDbusEvent::Toggle) = rx.next().await {
                (
                    Some((id, LauncherDbusEvent::Toggle)),
                    State::Waiting(conn, rx),
                )
            } else {
                (None, State::Finished)
            }
        }
        State::Finished => cosmic::iced::futures::future::pending().await,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LauncherDbusEvent {
    Toggle,
}

#[derive(Debug)]
pub(crate) struct CosmicLauncherServer {
    pub(crate) tx: UnboundedSender<LauncherDbusEvent>,
}

#[dbus_interface(name = "com.system76.CosmicLauncher")]
impl CosmicLauncherServer {
    async fn toggle(&self) {
        self.tx.unbounded_send(LauncherDbusEvent::Toggle).unwrap();
    }
}
