#![feature(const_fn, type_alias_impl_trait)]

pub mod client;
pub use client::Client;

mod error;
pub use error::{Error, Result};

mod frame;
pub use frame::{Frame, Opcode, Payload};

pub mod handshake;

mod socket;
pub use socket::WebSocket;

pub mod server;
pub use server::Server;
