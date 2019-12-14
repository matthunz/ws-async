use crate::{Frame, Payload};
use bytes::{Bytes, BytesMut};
use hyper::upgrade::Upgraded;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::sync::{mpsc, Mutex};

#[pin_project::pin_project]
struct Inner<T> {
    transport: T,
    read_buf: BytesMut,
    pending: Option<mpsc::UnboundedSender<std::io::Result<Bytes>>>,
}

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

        let used = loop {
            if inner.read_buf.len() < 32 {
                inner.read_buf.extend_from_slice(&[0; 32]);
            }

            let amt = inner.transport.read(&mut inner.read_buf).await?;
            if amt != 0 {
                break amt;
            }
        };

        let bytes = inner.read_buf.split_to(used).freeze();
        Ok(Some(bytes))
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
        let (_pending, payload) = Payload::shared(Shared {
            inner: self.shared.inner.clone(),
        });

        Ok(Some(Frame::binary(payload)))
    }
}

impl WebSocket {
    pub async fn upgrade(body: hyper::Body) -> hyper::Result<Self> {
        body.on_upgrade().await.map(Self::from_upgraded)
    }
}
