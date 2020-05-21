mod error;
pub use error::{Error, Result};

mod frame;
pub use frame::{Frame, Opcode, Payload};

pub mod handshake;

mod socket;
pub use socket::WebSocket;
