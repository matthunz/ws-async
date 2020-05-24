use crate::frame::{Frame, Opcode, Payload, Raw};
use bytes::{Buf, BufMut, BytesMut};
use futures::{pin_mut, ready, Sink, SinkExt, StreamExt};
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::stream::Stream;
use tokio::sync::Mutex;

mod shared;
pub(crate) use shared::Shared;
use shared::{Inner, Pending};

/// A websocket `Sink` and `Stream`.
pub struct Socket<T> {
    shared: Shared<T>,
    write_buf: BytesMut,
}

impl<T> Socket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    /// Creates a new `Socket` from the previously upgraded connection provided.
    /// ```
    /// # async {
    /// use tokio::net::TcpStream;
    /// use ws_async::Socket;
    ///
    /// let upgraded = TcpStream::connect("127.0.0.1:80").await?;
    /// // upgrade connection...
    ///
    /// let ws = Socket::from_upgraded(upgraded);
    /// # Ok::<_, Box<dyn std::error::Error>>(())
    /// # };
    /// ```
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

    /// Returns a future that sends a `Frame` on the socket to the remote address to which it is connected.
    /// ```
    /// use tokio::net::TcpStream;
    /// use ws_async::Socket;
    /// use ws_async::frame::Frame;
    ///
    /// async fn handle(ws: &mut Socket<TcpStream>) -> std::io::Result<()> {
    ///     let frame = Frame::text("Hello World!".as_bytes());
    ///     ws.send_frame(frame).await
    /// }
    /// ```
    pub async fn send_frame<B: Buf>(&mut self, frame: Frame<B>) -> io::Result<()> {
        self.send(Raw {
            frame,
            mask: None,
            finished: true,
        })
        .await
    }

    pub async fn send_masked<B: Buf>(
        &mut self,
        frame: Frame<B>,
        mask: Option<u32>,
    ) -> io::Result<()> {
        self.send(Raw {
            frame,
            mask,
            finished: true,
        })
        .await
    }

    pub async fn send_stream<B, P>(&mut self, frame: Frame<P>) -> io::Result<()>
    where
        P: Stream<Item = io::Result<B>> + Unpin,
        B: Buf,
    {
        let mut payload = frame.payload;
        let mut pending = None;

        loop {
            let buf = if let Some(pending) = pending.take() {
                pending
            } else {
                if let Some(bytes_res) = payload.next().await {
                    bytes_res?
                } else {
                    break;
                }
            };

            let new_frame = Frame::new(frame.opcode, frame.rsv, buf);
            let finished = if let Some(next) = payload.next().await {
                pending = Some(next?);
                false
            } else {
                true
            };

            self.send_frame(new_frame).await?;
            if finished {
                break;
            }
        }

        Ok(())
    }
}

impl<T, B> Sink<Raw<B>> for Socket<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
    B: Buf,
{
    type Error = io::Error;

    fn start_send(mut self: Pin<&mut Self>, raw: Raw<B>) -> Result<(), Self::Error> {
        let op = match raw.frame.opcode {
            Opcode::Continue => 0,
            Opcode::Text => 1,
            Opcode::Binary => 2,
            _ => todo!(),
        };
        self.write_buf.put_u8(1 << 7 | op);

        let payload = raw.frame.payload;
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
        Poll::Ready(Ok(()))
    }
}

impl<T> Stream for Socket<T>
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
                // TODO handle finished
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
