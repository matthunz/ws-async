use crate::{Frame, Payload};
use bytes::{Buf, Bytes, BytesMut};
use futures::{pin_mut, ready};
use hyper::upgrade::Upgraded;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::stream::Stream;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::{Mutex, MutexGuard};

pub(crate) struct Pending {
    sender: UnboundedSender<io::Result<Bytes>>,
    pub(crate) remaining: usize,
    pub(crate) mask: Option<[u8; 4]>,
}

#[pin_project::pin_project]
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
                    match transport.poll_read_buf(cx, &mut self.read_buf) {
                        Poll::Ready(Ok(used)) => {
                            len = used;
                        }
                        Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
                        Poll::Pending => return Poll::Pending,
                    }
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

    fn poll_read_buf(&mut self, cx: &mut Context) -> Poll<io::Result<usize>> {
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

pub struct WebSocket<T = Upgraded> {
    shared: Shared<T>,
}

impl<T> WebSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub fn from_upgraded(upgraded: T) -> Self {
        Self {
            shared: Shared {
                inner: Arc::new(Mutex::new(Inner {
                    transport: upgraded,
                    read_buf: BytesMut::new(),
                    pending: None,
                })),
            },
        }
    }
}

impl<T> Stream for WebSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    type Item = io::Result<Frame<Payload<T>>>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut inner = ready!(self.shared.poll_lock(cx));

        while let Some(res) = ready!(inner.poll_bytes(cx)) {
            if let Some(pending) = &mut inner.pending {
                pending.sender.send(res).unwrap();
            }
        }

        let mut f = ws_frame::Frame::empty();
        loop {
            let end = ready!(inner.poll_read_buf(cx))?;

            if let ws_frame::Status::Complete(used) = &f.decode(&inner.read_buf[..end]) {
                // advance buf before a panic
                inner.read_buf.advance(*used);

                let head = f.head.as_ref().unwrap();
                let (sender, payload) = Payload::shared(Shared {
                    inner: self.shared.inner.clone(),
                });

                inner.pending = Some(Pending {
                    sender,
                    remaining: f.payload_len.unwrap() as usize,
                    mask: f.mask,
                });

                break Poll::Ready(Some(Ok(Frame::new(head.op, head.rsv, payload))));
            }
        }
    }
}

impl WebSocket {
    pub async fn upgrade(body: hyper::Body) -> hyper::Result<Self> {
        body.on_upgrade()
            .await
            .map(Self::from_upgraded)
            .map_err(Into::into)
    }
}
