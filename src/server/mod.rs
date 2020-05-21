use crate::WebSocket;
use futures::future;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tokio::sync::mpsc::Receiver;
use tokio::task::{self, JoinHandle};

mod factory;
pub use factory::WsFactory;

mod service;
pub use service::WsService;

type UpgradeHandle = JoinHandle<hyper::Result<WebSocket>>;

/// A WebSocket socket server, listening for connections.
pub struct Server {
    rx: Receiver<UpgradeHandle>,
}

impl Server {
    /// Creates a new Server which will be bound to the specified address.
    pub fn bind(addr: &SocketAddr) -> hyper::Result<Self> {
        let http = hyper::Server::try_bind(addr)?;
        let (make_svc, rx) = WsFactory::new();

        task::spawn(http.serve(make_svc));
        Ok(Self { rx })
    }

    pub async fn next_socket(&mut self) -> hyper::Result<Option<WebSocket>> {
        if let Some(handle) = self.next_upgrade().await {
            // TODO don't unwrap
            let ws = handle.await.unwrap()?;
            Ok(Some(ws))
        } else {
            Ok(None)
        }
    }

    pub async fn next_upgrade(&mut self) -> Option<UpgradeHandle> {
        future::poll_fn(|cx| self.poll_upgrade(cx)).await
    }

    pub fn poll_upgrade(&mut self, cx: &mut Context) -> Poll<Option<UpgradeHandle>> {
        self.rx.poll_recv(cx)
    }
}
