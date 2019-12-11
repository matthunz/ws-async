#![feature(type_alias_impl_trait)]

mod frame;
pub use frame::{Frame, Opcode};

pub mod handshake;

mod payload;
pub use payload::Payload;

mod socket;
pub use socket::WebSocket;

pub mod server;
pub use server::Server;
