#![feature(type_alias_impl_trait)]

pub mod handshake;

mod socket;
pub use socket::WebSocket;

pub mod server;
pub use server::Server;
