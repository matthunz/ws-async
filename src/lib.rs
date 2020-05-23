mod error;
pub use error::{Error, Result};

pub mod frame;
//pub use frame::{Frame, Opcode, Payload, Masked};

pub mod handshake;

mod socket;
pub use socket::WebSocket;
