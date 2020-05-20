use crate::frame::Frame;
use crate::Payload;
use bytes::{Buf, Bytes, BytesMut};
use hyper::upgrade::Upgraded;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

#[derive(Debug)]
struct Pending {
    sender: UnboundedSender<io::Result<Bytes>>,
    remaining: usize,
    mask: Option<[u8; 4]>,
}

#[pin_project::pin_project]
#[derive(Debug)]
struct Inner<T> {
    transport: T,
    read_buf: BytesMut,
    pending: Option<Pending>,
}

#[derive(Debug)]
pub(crate) struct Shared<T> {
    inner: Arc<Mutex<Inner<T>>>,
}

impl<T> Shared<T>
where
    T: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn next_bytes(&mut self) -> io::Result<Option<Bytes>> {
        let mut g = self.inner.lock().await;
        let mut inner = Pin::new(&mut *g).project();
        let pending = inner
            .pending
            .as_mut()
            .expect("called `Payload::next_bytes` after `None`");

        if pending.remaining > 0 {
            let mut len = inner.read_buf.len();
            if len == 0 {
                loop {
                    let used = inner.transport.read_buf(&mut inner.read_buf).await?;
                    if used != 0 {
                        len = used;
                        break;
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

            Ok(Some(bytes.freeze()))
        } else {
            *inner.pending = None;
            Ok(None)
        }
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

    pub async fn next_frame(&mut self) -> io::Result<Option<Frame<Payload<T>>>> {
        let mut g = self.shared.inner.lock().await;
        let mut inner = Pin::new(&mut *g).project();

        if let Some(_pending) = inner.pending {
            todo!()
        } else {
            let mut f = ws_frame::Frame::empty();
            loop {
                let end = inner.transport.read_buf(&mut inner.read_buf).await?;

                if let ws_frame::Status::Complete(used) = &f.decode(&inner.read_buf[..end]) {
                    // advance buf before a panic
                    inner.read_buf.advance(*used);

                    let head = f.head.as_ref().unwrap();
                    let (sender, payload) = Payload::shared(Shared {
                        inner: self.shared.inner.clone(),
                    });

                    *inner.pending = Some(Pending {
                        sender,
                        remaining: f.payload_len.unwrap() as usize,
                        mask: f.mask,
                    });

                    break Ok(Some(Frame::new(head.op, head.rsv, payload)));
                }
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
