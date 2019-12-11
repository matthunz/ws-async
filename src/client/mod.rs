use crate::{handshake, WebSocket};
use hyper::client::conn::Builder;
use hyper::client::service::Connect;
use hyper::{Body, Request, Uri, header};
use tower_service::Service;

mod connect;
pub use connect::WsConnector;

pub struct Client<P> {
    http: Connect<WsConnector, P, Uri>,
}

impl Client<Body> {
    pub fn new() -> Self {
        let http = Connect::new(WsConnector::new(), Builder::new());
        Self { http }
    }
    pub async fn connect(&mut self, uri: Uri) -> hyper::Result<WebSocket> {
        let mut svc = self.http.call(uri).await?;

        // TODO don't unwrap
        let key = handshake::generate().await.unwrap();
        let req = Request::builder()
            .header(header::CONNECTION, header::UPGRADE)
            .header(header::UPGRADE, "websocket")
            .header("Sec-WebSocket-Key", &key)
            .body(Body::empty())
            .unwrap();
        let res = svc.call(req).await?;

        if let Some(accept) = handshake::get_accept(&res) {
            // TODO don't unwrap
            let clone = handshake::accept(&key).await.unwrap();

            if accept == &clone {
                res.into_body()
                    .on_upgrade()
                    .await
                    .map(WebSocket::from_upgraded)
            } else {
                unimplemented!()
            }
        } else {
            unimplemented!()
        }
    }
}
