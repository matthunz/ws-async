use hyper::header::HeaderValue;
use hyper::{Request, Response};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha1::{Digest, Sha1};
use std::convert::TryFrom;
use tokio::task::{self, JoinError};

unsafe fn encode_value(bytes: &[u8]) -> HeaderValue {
    match HeaderValue::try_from(base64::encode(&bytes)) {
        Ok(hv) => hv,
        Err(_) => unreachable!(),
    }
}

pub async fn generate() -> Result<HeaderValue, JoinError> {
    task::spawn_blocking(|| {
        let mut rng = thread_rng();
        let chars: String = std::iter::repeat(())
            .map(|()| rng.sample(Alphanumeric))
            .take(16)
            .collect();

        unsafe { encode_value(chars.as_bytes())}
    })
    .await
}

pub async fn accept(key: &HeaderValue) -> Result<HeaderValue, JoinError> {
    const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let bytes = [key.as_bytes(), GUID].concat();

    task::spawn_blocking(move || {
        let mut sha1 = Sha1::default();
        sha1.input(bytes);

        unsafe { encode_value(&sha1.result())}
    })
    .await
}

pub fn get_key<B>(req: &Request<B>) -> Option<&HeaderValue> {
    req.headers().get("Sec-WebSocket-Key")
}

pub fn get_accept<B>(res: &Response<B>) -> Option<&HeaderValue> {
    res.headers().get("Sec-WebSocket-Accept")
}
