use std::future::Future;

#[derive(Clone)]
pub struct UnsafeFuture<Fut> {
    inner: Fut,
}

impl<F> UnsafeFuture<F>
where
    F: Future,
{
    pub fn new(inner: F) -> Self {
        Self { inner }
    }
}

unsafe impl<F> Send for UnsafeFuture<F> where F: Future {}
unsafe impl<F> Sync for UnsafeFuture<F> where F: Future {}

impl<F> Future for UnsafeFuture<F>
where
    F: Future,
{
    type Output = F::Output;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let pinned_future = unsafe { self.as_mut().map_unchecked_mut(|this| &mut this.inner) };
        pinned_future.poll(cx)
    }
}
