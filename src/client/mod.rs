use crate::{handshake, WebSocket};
use hyper::client::conn::Builder;
use hyper::client::service::Connect;
use hyper::{header, Body, Request, Uri};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_service::Service;

mod connect;
pub use connect::WsConnector;

#[derive(Debug)]
pub struct Client<P = Body> {
    http: Connect<WsConnector, P, Uri>,
}

impl Default for Client {
    fn default() -> Self {
        Self::from(Builder::new())
    }
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn connect(&mut self, uri: Uri) -> hyper::Result<WebSocket> {
        self.call(uri).await
    }
}

impl<P> From<Builder> for Client<P> {
    fn from(builder: Builder) -> Self {
        let http = Connect::new(WsConnector::new(), builder);
        Self { http }
    }
}

impl Service<Uri> for Client {
    type Response = WebSocket;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = hyper::Result<Self::Response>> + Send + 'static>>;

    fn poll_ready(&mut self, _cx: &mut Context) -> Poll<hyper::Result<()>> {
        Ok(()).into()
    }

    fn call(&mut self, uri: Uri) -> Self::Future {
        let svc_fut = self.http.call(uri);

        Box::pin(async move {
            let mut svc = svc_fut.await?;

            // TODO don't unwrap
            let key = handshake::generate().await.unwrap();
            let req = Request::builder()
                .header(header::CONNECTION, header::UPGRADE)
                .header(header::UPGRADE, "websocket")
                .header(handshake::SEC_WEBSOCKET_KEY, &key)
                .body(Body::empty())
                .unwrap();

            let res = svc.call(req).await?;
            if let Some(accept) = res.headers().get(handshake::SEC_WEBSOCKET_ACCEPT) {
                // TODO don't unwrap
                let clone = handshake::accept(&key).await.unwrap();

                if accept == &clone {
                    WebSocket::upgrade(res.into_body()).await
                } else {
                    unimplemented!()
                }
            } else {
                unimplemented!()
            }
        })
    }
}
