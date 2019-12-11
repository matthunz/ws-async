use hyper::header::HeaderValue;
use hyper::Request;
use tokio::task::{self, JoinError};
use sha1::{Digest, Sha1};
use std::convert::TryFrom;

pub async fn accept(key: &HeaderValue) -> Result<HeaderValue, JoinError> {
    const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let bytes = [key.as_bytes(), GUID].concat();

    task::spawn_blocking(move || {
        let mut sha1 = Sha1::default();
        sha1.input(bytes);
        let encoded = base64::encode(&sha1.result());

        match HeaderValue::try_from(encoded) {
            Ok(hv) => hv,
            Err(_) => unreachable!(),
        }
    })
    .await
}

pub fn get_key<B>(req: &Request<B>) -> Option<&HeaderValue> {
    req.headers().get("Sec-WebSocket-Key")
}
