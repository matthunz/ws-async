use crate::socket::Shared;
use bytes::Bytes;
use futures::ready;
use hyper::upgrade::Upgraded;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::stream::Stream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub struct Payload<T = Upgraded> {
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

impl<T> Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    pub(crate) fn shared(shared: Shared<T>) -> (UnboundedSender<io::Result<Bytes>>, Self) {
        let (tx, pending) = mpsc::unbounded_channel();
        let me = Self { shared, pending };
        (tx, me)
    }
}
