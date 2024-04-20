use pin_project_lite::pin_project;

use std::error::Error as StdError;
use std::future::Future;
use std::pin::{pin, Pin};
use std::task::{Context, Poll};

use futures_util::ready;
use h3::server::builder;

use crate::body::{Body, Buf, Incoming as IncomingBody};
use crate::proto::Dispatched;
use crate::rt::Executor;
use crate::service::HttpService;
use crate::Response;

pin_project! {
    pub(crate) struct Server<E, S, C, B>
    where
        C: h3::quic::Connection<B>,
        <C as h3::quic::Connection<B>>::BidiStream: Send,
        <C as h3::quic::Connection<B>>::BidiStream: 'static,
        B: bytes::Buf,
    {
        exec: E,
        service: S,
        state: State<C, B>,
    }
}

enum State<C, B>
where
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf,
{
    Initiating(Initiating<C, B>),
    Serving(Serving<C, B>),
    Closed,
}

impl<E, S, C, B> Server<E, S, C, B>
where
    S: HttpService<IncomingBody, ResBody = B>,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    E: Executor<S::Future>,
    C: h3::quic::Connection<B> + 'static,
    <C as h3::quic::Connection<B>>::BidiStream: Send,
    <C as h3::quic::Connection<B>>::BidiStream: 'static,
    B: bytes::Buf + 'static,
{
    pub(crate) fn new(exec: E, service: S, conn: C) -> Server<E, S, C, B> {
        Server {
            exec,
            service,
            state: State::Initiating(Initiating::new(conn)),
        }
    }
}

impl<E, S, C, B> Future for Server<E, S, C, B>
where
    S: HttpService<IncomingBody, ResBody = B>,
    S::Error: Into<Box<dyn StdError + Send + Sync>>,
    E: Executor<S::Future>,
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf,
{
    type Output = crate::Result<Dispatched>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let me = &mut *self;
        loop {
            match me.state {
                State::Initiating(ref mut init) => {
                    let mut conn = ready!(Pin::new(init).poll(cx));
                    match conn {
                        Ok(conn) => {
                            me.state = State::Serving(Serving { conn });
                        }
                        Err(e) => {
                            todo!()
                            // return Poll::Ready(Err(e));
                        }
                    }
                }
                State::Serving(ref mut srv) => {
                    match srv.poll_server(cx, &mut me.service, &mut me.exec) {
                        Poll::Pending => return Poll::Pending,
                        Poll::Ready(_) => (),
                    };
                    return Poll::Ready(Ok(Dispatched::Shutdown));
                }
                _ => {
                    return Poll::Ready(Ok(Dispatched::Shutdown));
                }
            }
        }
    }
}

struct Serving<C, B>
where
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf,
{
    conn: h3::server::Connection<C, B>,
}

impl<C, B> Serving<C, B>
where
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf,
{
    fn poll_server<S, E>(
        &mut self,
        cx: &mut Context<'_>,
        service: &mut S,
        exec: &mut E,
    ) -> Poll<crate::Result<()>>
    where
        S: HttpService<IncomingBody, ResBody = B>,
        S::Error: Into<Box<dyn StdError + Send + Sync>>,
        E: crate::rt::Executor<S::Future>,
    {
        let mut fut = Box::pin(self.conn.accept());
        match Pin::new(&mut fut).poll(cx) {
            // Ok(Some((req, stream))) => {
            //     todo!()
            //     // let fut = service.call(req);
            //     // ready!(stream.finish());
            // }
            // Ok(None) => {
            //     return Poll::Ready(Ok(()));
            // }
            // Err(err) => {
            //     return Poll::Ready(Err(err));
            // }
            _ => {
                todo!()
            }
        }
    }
}

struct Initiating<C, B>
where
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf,
{
    fut: Pin<Box<dyn Future<Output = Result<h3::server::Connection<C, B>, h3::Error>>>>,
}

impl<C, B> Initiating<C, B>
where
    C: h3::quic::Connection<B> + 'static,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf + 'static,
{
    fn new(conn: C) -> Initiating<C, B> {
        let builder = h3::server::builder();
        let fut = async move { builder.build(conn).await };
        Initiating { fut: Box::pin(fut) }
    }
}

impl<C, B> Future for Initiating<C, B>
where
    C: h3::quic::Connection<B>,
    <C as h3::quic::Connection<B>>::BidiStream: Send + 'static,
    B: bytes::Buf,
{
    type Output = Result<h3::server::Connection<C, B>, h3::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.fut).poll(cx)
    }
}
