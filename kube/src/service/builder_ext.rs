// Borrowing from Tonic https://git.io/JtwWj
// MIT Copyright (c) 2020 Lucio Franco
use tower::{
    layer::{util::Stack, Layer},
    util::Either,
    ServiceBuilder,
};

pub(crate) trait ServiceBuilderExt<L> {
    fn optional_layer<T>(self, l: Option<T>) -> ServiceBuilder<Stack<OptionalLayer<T>, L>>;
}

impl<L> ServiceBuilderExt<L> for ServiceBuilder<L> {
    fn optional_layer<T>(self, inner: Option<T>) -> ServiceBuilder<Stack<OptionalLayer<T>, L>> {
        self.layer(OptionalLayer { inner })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OptionalLayer<L> {
    inner: Option<L>,
}

impl<S, L> Layer<S> for OptionalLayer<L>
where
    L: Layer<S>,
{
    type Service = Either<L::Service, S>;

    fn layer(&self, s: S) -> Self::Service {
        if let Some(inner) = &self.inner {
            Either::A(inner.layer(s))
        } else {
            Either::B(s)
        }
    }
}
