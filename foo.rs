#![feature(prelude_import)]
#[prelude_import]
use std::prelude::v1::*;
#[macro_use]
extern crate std;
mod watcher {
    use derivative::Derivative;
    use futures::{Future, Stream};
    use kube::{
        api::{ListParams, Meta, ObjectList, WatchEvent},
        Api,
    };
    use pin_project::{pin_project, project};
    use serde::de::DeserializeOwned;
    use snafu::{Backtrace, Snafu};
    use std::{
        clone::Clone,
        collections::VecDeque,
        pin::Pin,
        task::{Context, Poll},
    };
    pub enum Error {
        WatchStartFailed {
            source: kube::Error,
            backtrace: Backtrace,
        },
    }
    ///SNAFU context selector for the `Error::WatchStartFailed` variant
    struct WatchStartFailed;
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for WatchStartFailed {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match *self {
                WatchStartFailed => {
                    let mut debug_trait_builder = f.debug_tuple("WatchStartFailed");
                    debug_trait_builder.finish()
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::marker::Copy for WatchStartFailed {}
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::clone::Clone for WatchStartFailed {
        #[inline]
        fn clone(&self) -> WatchStartFailed {
            {
                *self
            }
        }
    }
    impl snafu::IntoError<Error> for WatchStartFailed
    where
        Error: snafu::Error + snafu::ErrorCompat,
    {
        type Source = kube::Error;
        fn into_error(self, error: Self::Source) -> Error {
            Error::WatchStartFailed {
                source: (|v| v)(error),
                backtrace: snafu::GenerateBacktrace::generate(),
            }
        }
    }
    #[allow(single_use_lifetimes)]
    impl core::fmt::Display for Error {
        fn fmt(&self, __snafu_display_formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
            #[allow(unused_variables)]
            match *self {
                Error::WatchStartFailed {
                    ref backtrace,
                    ref source,
                } => __snafu_display_formatter.write_fmt(::core::fmt::Arguments::new_v1(
                    &["WatchStartFailed: "],
                    &match (&source,) {
                        (arg0,) => [::core::fmt::ArgumentV1::new(
                            arg0,
                            ::core::fmt::Display::fmt,
                        )],
                    },
                )),
            }
        }
    }
    #[allow(single_use_lifetimes)]
    impl snafu::Error for Error
    where
        Self: core::fmt::Debug + core::fmt::Display,
    {
        fn description(&self) -> &str {
            match *self {
                Error::WatchStartFailed { .. } => "Error :: WatchStartFailed",
            }
        }
        fn cause(&self) -> Option<&dyn snafu::Error> {
            use snafu::AsErrorSource;
            match *self {
                Error::WatchStartFailed { ref source, .. } => {
                    core::option::Option::Some(source.as_error_source())
                }
            }
        }
        fn source(&self) -> Option<&(dyn snafu::Error + 'static)> {
            use snafu::AsErrorSource;
            match *self {
                Error::WatchStartFailed { ref source, .. } => {
                    core::option::Option::Some(source.as_error_source())
                }
            }
        }
    }
    #[allow(single_use_lifetimes)]
    impl snafu::ErrorCompat for Error {
        fn backtrace(&self) -> Option<&snafu::Backtrace> {
            match *self {
                Error::WatchStartFailed { ref backtrace, .. } => {
                    snafu::GenerateBacktrace::as_backtrace(backtrace)
                }
            }
        }
    }
    #[automatically_derived]
    #[allow(unused_qualifications)]
    impl ::core::fmt::Debug for Error {
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            match (&*self,) {
                (&Error::WatchStartFailed {
                    source: ref __self_0,
                    backtrace: ref __self_1,
                },) => {
                    let mut debug_trait_builder = f.debug_struct("WatchStartFailed");
                    let _ = debug_trait_builder.field("source", &&(*__self_0));
                    let _ = debug_trait_builder.field("backtrace", &&(*__self_1));
                    debug_trait_builder.finish()
                }
            }
        }
    }
    type Result<T, E = Error> = std::result::Result<T, E>;
    #[derivative(Debug)]
    #[pin(__private())]
    enum State<K: Meta + Clone> {
        Empty,
        InitListing {
            #[pin]
            #[derivative(Debug = "ignore")]
            list_fut: Box<dyn Future<Output = kube::Result<ObjectList<K>>>>,
        },
        InitListed {
            resource_version: String,
            queue: VecDeque<K>,
        },
        Watching {
            resource_version: String,
            #[derivative(Debug = "ignore")]
            stream: Box<dyn Stream<Item = kube::Result<WatchEvent<K>>>>,
        },
    }
    #[allow(unused_qualifications)]
    impl<K: Meta + Clone> ::std::fmt::Debug for State<K>
    where
        K: ::std::fmt::Debug,
    {
        fn fmt(&self, __f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
            match *self {
                State::Empty => {
                    let mut __debug_trait_builder = __f.debug_tuple("Empty");
                    __debug_trait_builder.finish()
                }
                State::InitListing {
                    list_fut: ref __arg_0,
                } => {
                    let mut __debug_trait_builder = __f.debug_struct("InitListing");
                    __debug_trait_builder.finish()
                }
                State::InitListed {
                    resource_version: ref __arg_0,
                    queue: ref __arg_1,
                } => {
                    let mut __debug_trait_builder = __f.debug_struct("InitListed");
                    let _ = __debug_trait_builder.field("resource_version", &__arg_0);
                    let _ = __debug_trait_builder.field("queue", &__arg_1);
                    __debug_trait_builder.finish()
                }
                State::Watching {
                    resource_version: ref __arg_0,
                    stream: ref __arg_1,
                } => {
                    let mut __debug_trait_builder = __f.debug_struct("Watching");
                    let _ = __debug_trait_builder.field("resource_version", &__arg_0);
                    __debug_trait_builder.finish()
                }
            }
        }
    }
    #[allow(clippy::mut_mut)]
    #[allow(dead_code)]
    enum __StateProjection<'pin, K: Meta + Clone>
    where
        State<K>: 'pin,
    {
        Empty,
        InitListing {
            list_fut:
                ::core::pin::Pin<&'pin mut (Box<dyn Future<Output = kube::Result<ObjectList<K>>>>)>,
        },
        InitListed {
            resource_version: &'pin mut (String),
            queue: &'pin mut (VecDeque<K>),
        },
        Watching {
            resource_version: &'pin mut (String),
            stream: &'pin mut (Box<dyn Stream<Item = kube::Result<WatchEvent<K>>>>),
        },
    }
    #[allow(dead_code)]
    enum __StateProjectionRef<'pin, K: Meta + Clone>
    where
        State<K>: 'pin,
    {
        Empty,
        InitListing {
            list_fut:
                ::core::pin::Pin<&'pin (Box<dyn Future<Output = kube::Result<ObjectList<K>>>>)>,
        },
        InitListed {
            resource_version: &'pin (String),
            queue: &'pin (VecDeque<K>),
        },
        Watching {
            resource_version: &'pin (String),
            stream: &'pin (Box<dyn Stream<Item = kube::Result<WatchEvent<K>>>>),
        },
    }
    impl<K: Meta + Clone> State<K> {
        fn project<'pin>(self: ::core::pin::Pin<&'pin mut Self>) -> __StateProjection<'pin, K> {
            unsafe {
                match self.get_unchecked_mut() {
                    State::Empty => __StateProjection::Empty,
                    State::InitListing { list_fut } => __StateProjection::InitListing {
                        list_fut: ::core::pin::Pin::new_unchecked(list_fut),
                    },
                    State::InitListed {
                        resource_version,
                        queue,
                    } => __StateProjection::InitListed {
                        resource_version,
                        queue,
                    },
                    State::Watching {
                        resource_version,
                        stream,
                    } => __StateProjection::Watching {
                        resource_version,
                        stream,
                    },
                }
            }
        }
        fn project_ref<'pin>(self: ::core::pin::Pin<&'pin Self>) -> __StateProjectionRef<'pin, K> {
            unsafe {
                match self.get_ref() {
                    State::Empty => __StateProjectionRef::Empty,
                    State::InitListing { list_fut } => __StateProjectionRef::InitListing {
                        list_fut: ::core::pin::Pin::new_unchecked(list_fut),
                    },
                    State::InitListed {
                        resource_version,
                        queue,
                    } => __StateProjectionRef::InitListed {
                        resource_version,
                        queue,
                    },
                    State::Watching {
                        resource_version,
                        stream,
                    } => __StateProjectionRef::Watching {
                        resource_version,
                        stream,
                    },
                }
            }
        }
    }
    #[allow(non_snake_case)]
    fn __unpin_scope_State() {
        struct __State<'pin, K: Meta + Clone> {
            __pin_project_use_generics: ::pin_project::__private::AlwaysUnpin<'pin, (K)>,
            __field0: Box<dyn Future<Output = kube::Result<ObjectList<K>>>>,
        }
        impl<'pin, K: Meta + Clone> ::core::marker::Unpin for State<K> where
            __State<'pin, K>: ::core::marker::Unpin
        {
        }
    }
    trait StateMustNotImplDrop {}
    #[allow(clippy::drop_bounds)]
    impl<T: ::core::ops::Drop> StateMustNotImplDrop for T {}
    #[allow(single_use_lifetimes)]
    impl<K: Meta + Clone> StateMustNotImplDrop for State<K> {}
    #[allow(single_use_lifetimes)]
    impl<K: Meta + Clone> ::pin_project::__private::PinnedDrop for State<K> {
        unsafe fn drop(self: ::core::pin::Pin<&mut Self>) {}
    }
    #[pin(__private())]
    pub struct Watcher<K: Meta + Clone> {
        api: Api<K>,
        list_params: ListParams,
        #[pin]
        state: State<K>,
    }
    #[allow(clippy::mut_mut)]
    #[allow(dead_code)]
    pub(crate) struct __WatcherProjection<'pin, K: Meta + Clone>
    where
        Watcher<K>: 'pin,
    {
        api: &'pin mut (Api<K>),
        list_params: &'pin mut (ListParams),
        state: ::core::pin::Pin<&'pin mut (State<K>)>,
    }
    #[allow(dead_code)]
    pub(crate) struct __WatcherProjectionRef<'pin, K: Meta + Clone>
    where
        Watcher<K>: 'pin,
    {
        api: &'pin (Api<K>),
        list_params: &'pin (ListParams),
        state: ::core::pin::Pin<&'pin (State<K>)>,
    }
    impl<K: Meta + Clone> Watcher<K> {
        pub(crate) fn project<'pin>(
            self: ::core::pin::Pin<&'pin mut Self>,
        ) -> __WatcherProjection<'pin, K> {
            unsafe {
                let Watcher {
                    api,
                    list_params,
                    state,
                } = self.get_unchecked_mut();
                __WatcherProjection {
                    api,
                    list_params,
                    state: ::core::pin::Pin::new_unchecked(state),
                }
            }
        }
        pub(crate) fn project_ref<'pin>(
            self: ::core::pin::Pin<&'pin Self>,
        ) -> __WatcherProjectionRef<'pin, K> {
            unsafe {
                let Watcher {
                    api,
                    list_params,
                    state,
                } = self.get_ref();
                __WatcherProjectionRef {
                    api,
                    list_params,
                    state: ::core::pin::Pin::new_unchecked(state),
                }
            }
        }
    }
    #[allow(single_use_lifetimes)]
    #[allow(non_snake_case)]
    #[deny(safe_packed_borrows)]
    fn __pin_project_assert_not_repr_packed_Watcher<K: Meta + Clone>(val: &Watcher<K>) {
        &val.api;
        &val.list_params;
        &val.state;
    }
    #[allow(non_snake_case)]
    fn __unpin_scope_Watcher() {
        pub struct __Watcher<'pin, K: Meta + Clone> {
            __pin_project_use_generics: ::pin_project::__private::AlwaysUnpin<'pin, (K)>,
            __field0: State<K>,
        }
        impl<'pin, K: Meta + Clone> ::core::marker::Unpin for Watcher<K> where
            __Watcher<'pin, K>: ::core::marker::Unpin
        {
        }
    }
    trait WatcherMustNotImplDrop {}
    #[allow(clippy::drop_bounds)]
    impl<T: ::core::ops::Drop> WatcherMustNotImplDrop for T {}
    #[allow(single_use_lifetimes)]
    impl<K: Meta + Clone> WatcherMustNotImplDrop for Watcher<K> {}
    #[allow(single_use_lifetimes)]
    impl<K: Meta + Clone> ::pin_project::__private::PinnedDrop for Watcher<K> {
        unsafe fn drop(self: ::core::pin::Pin<&mut Self>) {}
    }
    impl<K: Meta + Clone + DeserializeOwned + 'static> Watcher<K> {
        pub fn new(api: Api<K>, list_params: ListParams) -> Self {
            Self {
                api,
                list_params,
                state: State::Empty,
            }
        }
    }
    async fn list_owning_wrapper<K: Meta + Clone + DeserializeOwned>(
        api: Api<K>,
        lp: ListParams,
    ) -> kube::Result<ObjectList<K>> {
        api.list(&lp).await
    }
    impl<K: Meta + Clone + DeserializeOwned + 'static> Stream for Watcher<K> {
        type Item = Result<K>;
        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut this = self.as_mut().project();
            match this.state.as_mut().project() {
                __StateProjection::Empty => {
                    this.state.set(State::InitListing {
                        list_fut: Box::new(list_owning_wrapper(
                            this.api.clone(),
                            this.list_params.clone(),
                        )),
                    });
                    self.poll_next(cx)
                }
                __StateProjection::InitListing { mut list_fut } => Future::Poll::Pending,
                x => Poll::Pending,
            }
        }
    }
}
pub use watcher::Watcher;
