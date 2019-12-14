#![feature(const_fn)]

pub mod client;
pub use client::Client;

mod error;
pub use error::{Error, Result};

mod frame;
pub use frame::{Frame, Opcode};

pub mod handshake;

mod payload;
pub use payload::Payload;

mod socket;
pub use socket::WebSocket;

pub mod server;
pub use server::Server;
