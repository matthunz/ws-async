use crate::socket::Shared;
use bytes::{BufMut, Bytes, BytesMut};
use futures::ready;
use std::fmt;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::stream::{Stream, StreamExt};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub struct Payload<T> {
    shared: Shared<T>,
    pending: UnboundedReceiver<io::Result<Bytes>>,
}

impl<T> Stream for Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(pending) = self.pending.try_recv() {
            Poll::Ready(Some(pending))
        } else {
            let mut inner = ready!(self.shared.poll_lock(cx));
            inner.poll_bytes(cx)
        }
    }
}

impl<T> fmt::Debug for Payload<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Payload").finish()
    }
}

impl<T> Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn bytes(&mut self) -> io::Result<Bytes> {
        let mut buf = BytesMut::new();
        while let Some(res) = self.next().await {
            let bytes = res?;
            buf.put(bytes);
        }
        Ok(buf.freeze())
    }

    #[inline]
    pub(crate) fn shared(shared: Shared<T>) -> (UnboundedSender<io::Result<Bytes>>, Self) {
        let (tx, pending) = mpsc::unbounded_channel();
        let me = Self { shared, pending };
        (tx, me)
    }
}
