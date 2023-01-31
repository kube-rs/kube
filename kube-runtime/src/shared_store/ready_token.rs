use tokio_util::sync::{CancellationToken, WaitForCancellationFuture};

// Todo - Don't use a CancellationToken to track this, either find something better or write it
/// A token that can be used to track readiness of multiple components.
#[derive(Clone, Default)]
pub struct ReadyToken(CancellationToken);

impl ReadyToken {
    pub fn new() -> Self {
        Self(CancellationToken::new())
    }

    pub fn is_ready(&self) -> bool {
        self.0.is_cancelled()
    }

    pub fn ready(&self) -> WaitForCancellationFuture {
        self.0.cancelled()
    }

    pub fn make_ready(&self) {
        self.0.cancel()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::FutureExt;

    #[test]
    fn it_is_not_ready_on_init() {
        let rt = ReadyToken::new();

        assert!(!rt.is_ready());
    }

    #[test]
    fn it_is_ready_after_make_ready() {
        let rt = ReadyToken::new();
        rt.make_ready();

        assert!(rt.is_ready());
    }

    #[test]
    fn it_does_not_resolve_ready_until_ready() {
        let rt = ReadyToken::new();

        let mut fut = rt.ready().boxed();
        assert!((&mut fut).now_or_never().is_none());

        rt.make_ready();
        assert!((&mut fut).now_or_never().is_some())
    }

    #[test]
    fn it_allows_multiple_futures_to_wait_to_be_ready() {
        let rt = ReadyToken::new();

        let mut fut1 = rt.ready().boxed();
        let mut fut2 = rt.ready().boxed();
        assert!((&mut fut1).now_or_never().is_none());
        assert!((&mut fut2).now_or_never().is_none());

        rt.make_ready();
        assert!((&mut fut1).now_or_never().is_some());
        assert!((&mut fut2).now_or_never().is_some());
    }
}
