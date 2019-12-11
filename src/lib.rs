#![feature(type_alias_impl_trait)]

use futures::{future, TryFutureExt};
use hyper::{Body, Request, Response};
use std::future::Future;
use std::net::SocketAddr;
use std::task::{Context, Poll};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::{self, JoinHandle};
use tower_service::Service;

pub type UpgradeHandle = JoinHandle<hyper::Result<WebSocket>>;

pub struct WebSocket;

pub struct WsFactory {
    tx: Sender<UpgradeHandle>,
}

impl WsFactory {
    pub fn new() -> (Self, Receiver<UpgradeHandle>) {
        let (tx, rx) = mpsc::channel(1);
        (Self { tx }, rx)
    }
}

impl<T> Service<T> for WsFactory {
    type Response = WsService;
    type Error = hyper::Error;
    type Future = future::Ready<hyper::Result<Self::Response>>;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<hyper::Result<()>> {
        Ok(()).into()
    }

    fn call(&mut self, _req: T) -> Self::Future {
        future::ready(Ok(WsService::new(self.tx.clone())))
    }
}

pub struct WsService {
    tx: Sender<UpgradeHandle>,
}

impl WsService {
    pub fn new(tx: Sender<UpgradeHandle>) -> Self {
        Self { tx }
    }
}

impl Service<Request<Body>> for WsService {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = impl Future<Output = hyper::Result<Self::Response>>;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<hyper::Result<()>> {
        Ok(()).into()
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let handle = task::spawn(req.into_body().on_upgrade().map_ok(|_| WebSocket));
        let mut tx = self.tx.clone();

        async move {
            tx.send(handle).await;
            Ok(Response::new(Body::empty()))
        }
    }
}

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
