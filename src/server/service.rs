use super::UpgradeHandle;
use crate::WebSocket;
use futures::TryFutureExt;
use hyper::{Body, Request, Response};
use std::future::Future;

use std::task::{Context, Poll};
use tokio::sync::mpsc::Sender;
use tower_service::Service;

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
        let handle = tokio::task::spawn(
            req.into_body()
                .on_upgrade()
                .map_ok(WebSocket::from_upgraded),
        );
        let mut tx = self.tx.clone();

        async move {
            tx.send(handle).await;
            Ok(Response::new(Body::empty()))
        }
    }
}
