use core::{
    future::Future,
    ops::DerefMut,
    pin::Pin,
    task::{Context, Poll},
};

use futures_core::{Stream, TryStream};
use pin_project_lite::pin_project;

pub trait PollClose: TryStream {
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
}

impl<P: Unpin + DerefMut<Target: PollClose<Error = Self::Error>>> PollClose for Pin<P>
where
    Self: TryStream,
{
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().as_mut().poll_close(cx)
    }
}

impl<'a, S: Unpin + PollClose<Error = Self::Error>> PollClose for &'a mut S
where
    Self: TryStream,
{
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        S::poll_close(Pin::new(self.get_mut()), cx)
    }
}

pin_project! {
    pub struct Closable<S: ?Sized> {
        #[pin]
        stream: S,
    }
}

impl<S> From<S> for Closable<S> {
    fn from(stream: S) -> Self {
        Self { stream }
    }
}

impl<S: ?Sized + TryStream> Stream for Closable<S> {
    type Item = Result<S::Ok, S::Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.project().stream.try_poll_next(cx)
    }
}

impl<A: ?Sized + TryStream> PollClose for Closable<A> {
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub struct Close<'a, S: ?Sized> {
    stream: &'a mut S,
}

impl<'a, S: ?Sized + Unpin + PollClose> Future for Close<'a, S> {
    type Output = Result<(), S::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        S::poll_close(Pin::new(self.get_mut().stream), cx)
    }
}

pub trait PollCloseExt: Unpin + PollClose {
    fn close(&mut self) -> Close<'_, Self> {
        Close { stream: self }
    }
}

impl<S: ?Sized + Unpin + PollClose> PollCloseExt for S {}
