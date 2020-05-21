use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use sha1::{Digest, Sha1};

pub const SEC_WEBSOCKET_ACCEPT: &str = "Sec-WebSocket-Accept";
pub const SEC_WEBSOCKET_KEY: &str = "Sec-WebSocket-Key";

pub fn generate() -> String {
    let mut rng = thread_rng();
    let chars: String = std::iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(16)
        .collect();

    base64::encode(chars.as_bytes())
}

pub fn accept(key: &[u8]) -> String {
    const GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let bytes = [key, GUID].concat();

    let mut sha1 = Sha1::default();
    sha1.input(bytes);

    base64::encode(&sha1.result())
}
