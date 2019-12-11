use tokio::io::{AsyncRead, AsyncWrite};

pub struct WebSocket<T = hyper::upgrade::Upgraded> {
    transport: T,
}

impl<T> WebSocket<T>
where
    T: AsyncRead + AsyncWrite,
{
    pub fn from_upgraded(upgraded: T) -> Self {
        Self {
            transport: upgraded,
        }
    }
}
