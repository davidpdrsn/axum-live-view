// vendored to not depend on tokio-stream, which depends on an outdated version of tokio-util

use futures_util::{ready, Stream};
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

pub(crate) trait StreamExt: Stream {
    fn merge<U>(self, other: U) -> Merge<Self, U>
    where
        U: Stream<Item = Self::Item>,
        Self: Sized,
    {
        Merge::new(self, other)
    }
}

impl<T: ?Sized> StreamExt for T where T: Stream {}

pin_project! {
    /// Stream returned by [`fuse()`][super::StreamExt::fuse].
    #[derive(Debug)]
    pub struct Fuse<T> {
        #[pin]
        stream: Option<T>,
    }
}

impl<T> Fuse<T>
where
    T: Stream,
{
    pub(crate) fn new(stream: T) -> Fuse<T> {
        Fuse {
            stream: Some(stream),
        }
    }
}

impl<T> Stream for Fuse<T>
where
    T: Stream,
{
    type Item = T::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        let res = match Option::as_pin_mut(self.as_mut().project().stream) {
            Some(stream) => ready!(stream.poll_next(cx)),
            None => return Poll::Ready(None),
        };

        if res.is_none() {
            // Do not poll the stream anymore
            self.as_mut().project().stream.set(None);
        }

        Poll::Ready(res)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.stream {
            Some(ref stream) => stream.size_hint(),
            None => (0, Some(0)),
        }
    }
}

pin_project! {
    /// Stream returned by the [`merge`](super::StreamExt::merge) method.
    pub struct Merge<T, U> {
        #[pin]
        a: Fuse<T>,
        #[pin]
        b: Fuse<U>,
        // When `true`, poll `a` first, otherwise, `poll` b`.
        a_first: bool,
    }
}

impl<T, U> Merge<T, U> {
    pub(super) fn new(a: T, b: U) -> Merge<T, U>
    where
        T: Stream,
        U: Stream,
    {
        Merge {
            a: Fuse::new(a),
            b: Fuse::new(b),
            a_first: true,
        }
    }
}

impl<T, U> Stream for Merge<T, U>
where
    T: Stream,
    U: Stream<Item = T::Item>,
{
    type Item = T::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T::Item>> {
        let me = self.project();
        let a_first = *me.a_first;

        // Toggle the flag
        *me.a_first = !a_first;

        if a_first {
            poll_next(me.a, me.b, cx)
        } else {
            poll_next(me.b, me.a, cx)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        merge_size_hints(self.a.size_hint(), self.b.size_hint())
    }
}

fn poll_next<T, U>(
    first: Pin<&mut T>,
    second: Pin<&mut U>,
    cx: &mut Context<'_>,
) -> Poll<Option<T::Item>>
where
    T: Stream,
    U: Stream<Item = T::Item>,
{
    use Poll::*;

    let mut done = true;

    match first.poll_next(cx) {
        Ready(Some(val)) => return Ready(Some(val)),
        Ready(None) => {}
        Pending => done = false,
    }

    match second.poll_next(cx) {
        Ready(Some(val)) => return Ready(Some(val)),
        Ready(None) => {}
        Pending => done = false,
    }

    if done {
        Ready(None)
    } else {
        Pending
    }
}

fn merge_size_hints(
    (left_low, left_high): (usize, Option<usize>),
    (right_low, right_hign): (usize, Option<usize>),
) -> (usize, Option<usize>) {
    let low = left_low.saturating_add(right_low);
    let high = match (left_high, right_hign) {
        (Some(h1), Some(h2)) => h1.checked_add(h2),
        _ => None,
    };
    (low, high)
}
