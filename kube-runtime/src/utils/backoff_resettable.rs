use std::{ops::DerefMut, time::Duration};

use backon::{Backoff, BackoffBuilder};

/// A [`Backoff`] that can also be reset.
///
/// Implemented by [`ResettableBackoffWrapper`].
// Separated into a trait so that it can be used as a trait object, erasing the backing [`BackoffBuilder`].
pub trait ResettableBackoff: Backoff {
    fn reset(&mut self);
}

impl ResettableBackoff for Box<dyn ResettableBackoff + Send> {
    fn reset(&mut self) {
        Box::deref_mut(self).reset();
    }
}

/// Implements [`ResettableBackoff`] by reconstructing the backing [`Backoff`] each time [`Self::reset`] has been called.
#[derive(Debug)]
pub struct ResettableBackoffWrapper<B: BackoffBuilder> {
    backoff_builder: B,
    current_backoff: Option<B::Backoff>,
}

impl<B: BackoffBuilder> ResettableBackoffWrapper<B> {
    pub fn new(backoff_builder: B) -> Self {
        Self {
            backoff_builder,
            current_backoff: None,
        }
    }
}

impl<B: BackoffBuilder + Default> Default for ResettableBackoffWrapper<B> {
    fn default() -> Self {
        Self::new(B::default())
    }
}

impl<B: BackoffBuilder + Clone> Iterator for ResettableBackoffWrapper<B> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.current_backoff
            .get_or_insert_with(|| self.backoff_builder.clone().build())
            .next()
    }
}

impl<B: BackoffBuilder + Clone> ResettableBackoff for ResettableBackoffWrapper<B> {
    fn reset(&mut self) {
        self.current_backoff = None;
    }
}
