//! HTTP/3 Server Connections

use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::rt::{Read, Write};
use futures_util::ready;
use pin_project_lite::pin_project;

use crate::body::{Body, Incoming as IncomingBody};
use crate::proto;
use crate::rt::Executor;
use crate::service::HttpService;
use crate::{common::time::Time, rt::Timer};

pin_project! {
    /// bla bla
    #[must_use = "futures do nothing unless polled"]
    pub struct Connection<E, S, C, B>
    where
        S: HttpService<IncomingBody, ResBody = B>,
        C: h3::quic::Connection<B>,
        <C as h3::quic::Connection<B>>::BidiStream: Send,
        <C as h3::quic::Connection<B>>::BidiStream: 'static,
        B: bytes::Buf,
    {
        conn: proto::h3::server::Server<E, S, C, B>,
    }
}

impl<E, S, C, B> Future for Connection<E, S, C, B>
where
    S: HttpService<IncomingBody, ResBody = B>,
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send,
    <C as h3::quic::Connection<B>>::BidiStream: 'static,
    B: bytes::Buf,
    E: Executor<S::Future>,
{
    type Output = crate::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match ready!(Pin::new(&mut self.conn).poll(cx)) {
            Ok(_done) => Poll::Ready(Ok(())),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}
