use crate::frame::{Frame, Masked, Opcode, Payload};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use futures::{pin_mut, ready, Sink, SinkExt};
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

pub struct WebSocket<T> {
    shared: Shared<T>,
    write_buf: BytesMut,
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
            write_buf: BytesMut::new(),
        }
    }

    pub async fn send_frame<B>(&mut self, frame: Frame<B>) -> io::Result<()>
    where
        B: Buf,
    {
        self.send(Masked::new(frame, None)).await
    }
}

impl<T, B> Sink<Masked<B>> for WebSocket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
    B: Buf,
{
    type Error = io::Error;

    fn start_send(mut self: Pin<&mut Self>, masked: Masked<B>) -> Result<(), Self::Error> {
        let op = match masked.frame.opcode() {
            Opcode::Continue => 0,
            Opcode::Text => 1,
            Opcode::Binary => 2,
            _ => todo!(),
        };
        self.write_buf.put_u8(1 << 7 | op);

        let payload = masked.frame.into_payload();

        let mask = 0;
        let mut put_second = |payload_len: u8| self.write_buf.put_u8(mask << 7 | payload_len);
        let len = payload.bytes().len();
        if len > 127 {
            if len > 127 + u32::MAX as usize {
                put_second(127);
                self.write_buf.put_u64((len - 127) as _);
            } else {
                put_second(126);
                self.write_buf.put_u32((len - 127) as _);
            }
        } else {
            put_second(len as _);
        }

        self.write_buf.put(payload);

        Ok(())
    }

    fn poll_ready(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let Self {
            shared,
            ref mut write_buf,
        } = &mut *self;
        let mut inner = ready!(shared.poll_lock(cx));

        while write_buf.len() > 0 {
            let transport = &mut inner.transport;
            pin_mut!(transport);
            ready!(transport.poll_write_buf(cx, write_buf))?;
        }

        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        todo!()
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
