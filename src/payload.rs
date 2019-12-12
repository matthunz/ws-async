use crate::socket::Shared;
use bytes::Bytes;
use futures::Stream;
use hyper::upgrade::Upgraded;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

enum Kind<T> {
    Once(Option<Bytes>),
    Shared {
        socket: Shared<T>,
        pending: UnboundedReceiver<io::Result<Bytes>>,
    },
}

pub struct Payload<T = Upgraded> {
    kind: Kind<T>,
}

impl Payload {
    pub const fn empty() -> Self {
        Self::new(Kind::Once(None))
    }
}

impl<T> Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn next_bytes(&mut self) -> io::Result<Option<Bytes>> {
        match &mut self.kind {
            Kind::Once(once) => Ok(once.take()),
            Kind::Shared { socket, pending } => {
                if let Some(res) = pending.recv().await {
                    Ok(Some(res?))
                } else {
                    socket.next_bytes().await
                }
            }
        }
    }
    pub fn stream(&mut self) -> impl Stream<Item = io::Result<Bytes>> + '_ {
        async_stream::try_stream! {
            while let Some(bytes) = self.next_bytes().await? {
                yield bytes;
            }
        }
    }
    const fn new(kind: Kind<T>) -> Self {
        Self { kind }
    }
    #[inline]
    pub(crate) fn shared(socket: Shared<T>) -> (UnboundedSender<io::Result<Bytes>>, Self) {
        let (tx, pending) = mpsc::unbounded_channel();
        let me = Self::new(Kind::Shared { socket, pending });
        (tx, me)
    }
}

impl Default for Payload {
    #[inline]
    fn default() -> Self {
        Self::empty()
    }
}

impl From<Bytes> for Payload {
    #[inline]
    fn from(bytes: Bytes) -> Self {
        if bytes.is_empty() {
            Self::empty()
        } else {
            Self::new(Kind::Once(Some(bytes)))
        }
    }
}
