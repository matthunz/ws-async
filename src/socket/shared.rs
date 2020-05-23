use bytes::{Bytes, BytesMut};
use futures::{pin_mut, ready};
use std::future::Future;
use std::io;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, MutexGuard};

pub(crate) struct Pending {
    pub(crate) sender: UnboundedSender<io::Result<Bytes>>,
    pub(crate) remaining: usize,
    pub(crate) mask: Option<[u8; 4]>,
}

pub(crate) struct Inner<T> {
    pub(crate) transport: T,
    pub(crate) read_buf: BytesMut,
    pub(crate) pending: Option<Pending>,
}

impl<T> Inner<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn poll_bytes(&mut self, cx: &mut Context) -> Poll<Option<io::Result<Bytes>>> {
        if let Some(pending) = &mut self.pending {
            if pending.remaining > 0 {
                let mut len = self.read_buf.len();
                while len == 0 {
                    let transport = &mut self.transport;
                    pin_mut!(transport);
                    len = ready!(transport.poll_read_buf(cx, &mut self.read_buf))?;
                }

                let mut bytes = self.read_buf.split_to(len.min(pending.remaining));
                pending.remaining = pending.remaining.saturating_sub(len);

                if let Some(mask) = pending.mask {
                    for (i, b) in bytes.iter_mut().enumerate() {
                        *b ^= mask[i % 4];
                    }
                }

                return Poll::Ready(Some(Ok(bytes.freeze())));
            } else {
                self.pending = None;
            }
        }

        Poll::Ready(None)
    }

    pub fn poll_read_buf(&mut self, cx: &mut Context) -> Poll<io::Result<usize>> {
        let transport = &mut self.transport;
        pin_mut!(transport);
        transport.poll_read_buf(cx, &mut self.read_buf)
    }
}

pub(crate) struct Shared<T> {
    pub(crate) inner: Arc<Mutex<Inner<T>>>,
}

impl<T> Shared<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn poll_lock(&self, cx: &mut Context) -> Poll<MutexGuard<Inner<T>>> {
        let fut = self.inner.lock();
        pin_mut!(fut);
        fut.poll(cx)
    }
}
