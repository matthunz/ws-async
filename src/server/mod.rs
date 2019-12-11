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

pub struct Server {
    rx: Receiver<UpgradeHandle>,
}

impl Server {
    pub fn bind(addr: &SocketAddr) -> Self {
        let (make_svc, rx) = WsFactory::new();
        task::spawn(hyper::Server::bind(addr).serve(make_svc));

        Self { rx }
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
