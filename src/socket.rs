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
pub(crate) struct Pending {
    sender: UnboundedSender<io::Result<Bytes>>,
    pub(crate) remaining: usize,
    pub(crate) mask: Option<[u8; 4]>,
}

#[pin_project::pin_project]
#[derive(Debug)]
pub(crate) struct Inner<T> {
    pub(crate) transport: T,
    pub(crate) read_buf: BytesMut,
    pub(crate) pending: Option<Pending>,
}

#[derive(Debug)]
pub(crate) struct Shared<T> {
    pub(crate) inner: Arc<Mutex<Inner<T>>>,
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
