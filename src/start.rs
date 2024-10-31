use core::{
    future::Future,
    ops::DerefMut,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures_core::{Stream, TryStream};
use pin_project_lite::pin_project;

pub trait StartClose: Stream {
    fn start_close(self: Pin<&mut Self>);

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>
    where
        Self: TryStream,
    {
        self.as_mut().start_close();
        while ready!(self.as_mut().try_poll_next(cx)?).is_some() {}
        Poll::Ready(Ok(()))
    }
}

impl<P: Unpin + DerefMut<Target: StartClose>> StartClose for Pin<P> {
    fn start_close(self: Pin<&mut Self>) {
        self.get_mut().as_mut().start_close();
    }
}

impl<'a, S: Unpin + StartClose> StartClose for &'a mut S {
    fn start_close(self: Pin<&mut Self>) {
        S::start_close(Pin::new(self.get_mut()))
    }
}

pin_project! {
    pub struct Closable<S> {
        #[pin]
        stream: Option<S>,
    }
}

impl<S> From<S> for Closable<S> {
    fn from(stream: S) -> Self {
        Self {
            stream: Some(stream),
        }
    }
}

impl<S: TryStream> Stream for Closable<S> {
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project().stream.as_pin_mut() {
            Some(stream) => stream.try_poll_next(cx),
            None => Poll::Ready(None),
        }
    }
}

pub struct Close<'a, S: ?Sized> {
    stream: &'a mut S,
}

impl<'a, S: ?Sized + Unpin + StartClose + TryStream> Future for Close<'a, S> {
    type Output = Result<(), S::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        while ready!(S::try_poll_next(Pin::new(self.as_mut().stream), cx)?).is_some() {}
        Poll::Ready(Ok(()))
    }
}

pub trait StartCloseExt: Unpin + StartClose {
    fn close(&mut self) -> Close<'_, Self> {
        Self::start_close(Pin::new(self));
        Close { stream: self }
    }
}

impl<S: ?Sized + Unpin + StartClose> StartCloseExt for S {}
