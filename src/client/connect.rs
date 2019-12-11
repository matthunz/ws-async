use hyper::client::connect::dns::{GaiResolver, Name};
use hyper::client::HttpConnector;
use hyper::Uri;
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::net::TcpStream;
use tower_service::Service;

type StdError = Box<dyn std::error::Error + Send + Sync>;

pub struct WsConnector<R = GaiResolver> {
    http: HttpConnector<R>,
}

impl WsConnector {
    pub fn new() -> Self {
        Self::new_with_resolver(GaiResolver::new())
    }
}

impl<R> WsConnector<R>
where
    R: Service<Name> + Clone + Send + Sync + 'static,
    R::Response: Iterator<Item = IpAddr>,
    R::Error: Into<StdError>,
    R::Future: Send,
{
    pub fn new_with_resolver(resolver: R) -> Self {
        let mut http = HttpConnector::new_with_resolver(resolver);
        http.enforce_http(false);

        Self::from_http(http)
    }
    pub fn from_http(http: HttpConnector<R>) -> Self {
        Self { http }
    }
}

impl<R> Service<Uri> for WsConnector<R>
where
    R: Service<Name> + Clone + Send + Sync + 'static,
    R::Response: Iterator<Item = IpAddr>,
    R::Error: Into<StdError>,
    R::Future: Send,
{
    type Response = TcpStream;
    type Error = <HttpConnector<R> as Service<Uri>>::Error;
    type Future = <HttpConnector<R> as Service<Uri>>::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, dst: Uri) -> Self::Future {
        self.http.call(dst)
    }
}
