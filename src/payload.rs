use crate::socket::Shared;
use bytes::Bytes;
use hyper::upgrade::Upgraded;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

enum Kind<T> {
    Shared {
        socket: Shared<T>,
        pending: UnboundedReceiver<io::Result<Bytes>>,
    },
}

pub struct Payload<T = Upgraded> {
    kind: Kind<T>,
}

impl<T> Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub(crate) fn shared(socket: Shared<T>) -> (UnboundedSender<io::Result<Bytes>>, Self) {
        let (tx, pending) = mpsc::unbounded_channel();
        let me = Self {
            kind: Kind::Shared { socket, pending },
        };
        (tx, me)
    }
    pub async fn next_bytes(&mut self) -> io::Result<Option<Bytes>> {
        match &mut self.kind {
            Kind::Shared { socket, pending } => {
                if let Some(res) = pending.recv().await {
                    Ok(Some(res?))
                } else {
                    socket.next_bytes().await
                }
            }
        }
    }
}
