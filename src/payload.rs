use crate::socket::Shared;
use bytes::Bytes;
use futures::pin_mut;
use hyper::upgrade::Upgraded;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::stream::Stream;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

pub struct Payload<T = Upgraded> {
    socket: Shared<T>,
    pending: UnboundedReceiver<io::Result<Bytes>>,
}

impl<T> Stream for Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = io::Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Ok(res) = self.pending.try_recv() {
            Poll::Ready(Some(res))
        } else {
            let fut = self.socket.inner.lock();
            pin_mut!(fut);
            if let Poll::Ready(mut guard) = fut.poll(cx) {
                let mut inner = Pin::new(&mut *guard).project();
                let pending = inner
                    .pending
                    .as_mut()
                    .expect("called `Payload::next_bytes` after `None`");

                if pending.remaining > 0 {
                    let mut len = inner.read_buf.len();
                    if len == 0 {
                        loop {
                            let transport = &mut inner.transport;
                            pin_mut!(transport);
                            match transport.poll_read_buf(cx, &mut inner.read_buf) {
                                Poll::Ready(Ok(used)) if used != 0 => {
                                    len = used;
                                    break;
                                }
                                Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                                Poll::Pending => return Poll::Pending,
                                _ => {}
                            }
                        }
                    }

                    let mut bytes = inner.read_buf.split_to(len.min(pending.remaining));
                    pending.remaining = pending.remaining.saturating_sub(len);

                    if let Some(mask) = pending.mask {
                        for (i, b) in bytes.iter_mut().enumerate() {
                            *b ^= mask[i % 4];
                        }
                    }

                    Poll::Ready(Some(Ok(bytes.freeze())))
                } else {
                    *inner.pending = None;
                    Poll::Ready(None)
                }
            } else {
                Poll::Pending
            }
        }
    }
}

impl<T> Payload<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    #[inline]
    pub(crate) fn shared(socket: Shared<T>) -> (UnboundedSender<io::Result<Bytes>>, Self) {
        let (tx, pending) = mpsc::unbounded_channel();
        let me = Self { socket, pending };
        (tx, me)
    }
}
