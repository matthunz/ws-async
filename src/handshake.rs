use hyper::header::HeaderValue;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha1::{Digest, Sha1};
use std::convert::TryFrom;

pub const SEC_WEBSOCKET_ACCEPT: &str = "Sec-WebSocket-Accept";
pub const SEC_WEBSOCKET_KEY: &str = "Sec-WebSocket-Key";

unsafe fn encode_value(bytes: &[u8]) -> HeaderValue {
    match HeaderValue::try_from(base64::encode(&bytes)) {
        Ok(hv) => hv,
        Err(_) => unreachable!(),
    }
}

pub fn generate() -> HeaderValue {
    let mut rng = thread_rng();
    let chars: String = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect();

    unsafe { encode_value(chars.as_bytes()) }
}

pub fn accept(key: &HeaderValue) -> HeaderValue {
    const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let bytes = [key.as_bytes(), GUID].concat();

    let mut sha1 = Sha1::default();
    sha1.input(bytes);

    unsafe { encode_value(&sha1.result()) }
}
