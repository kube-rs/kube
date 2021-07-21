use std::ops::Deref;

use futures::TryFuture;
use http::{Request, Response};
use hyper::Body;
use kube_core::{Resource, WatchEvent};
use serde::{de::DeserializeOwned, Deserialize};
use snafu::{ResultExt, Snafu};

use crate::client::{
    decoder::{DecodeSingle, DecodeStream},
    scope::{self, NativeScope},
    Config,
};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("verb created invalid http request: {}", source))]
    BuildRequestFailed { source: http::Error },
}
type Result<T, E = Error> = std::result::Result<T, E>;

pub trait Verb {
    type ResponseDecoder: TryFuture + From<Response<Body>>;

    fn to_http_request(&self) -> Result<Request<Body>>;
}

pub struct Get<Kind: Resource, Scope> {
    pub name: String,
    pub scope: Scope,
    pub dyn_type: Kind::DynamicType,
}
impl<Kind: Resource + DeserializeOwned, Scope: NativeScope<Kind>> Verb for Get<Kind, Scope> {
    type ResponseDecoder = DecodeSingle<Kind>;

    fn to_http_request(&self) -> Result<Request<Body>> {
        Request::get(format!(
            "{}/{}",
            Kind::url_path(&self.dyn_type, self.scope.namespace()),
            self.name
        ))
        .body(Body::empty())
        .context(BuildRequestFailed)
    }
}

pub struct List<Kind: Resource, Scope> {
    pub scope: Scope,
    pub dyn_type: Kind::DynamicType,
}
impl<Kind: Resource + DeserializeOwned, Scope: scope::Scope> Verb for List<Kind, Scope> {
    type ResponseDecoder = DecodeSingle<ObjectList<Kind>>;

    fn to_http_request(&self) -> Result<Request<Body>> {
        Request::get(Kind::url_path(&self.dyn_type, self.scope.namespace()))
            .body(Body::empty())
            .context(BuildRequestFailed)
    }
}
#[derive(Deserialize)]
pub struct ObjectList<Kind> {
    pub items: Vec<Kind>,
}

pub struct Watch<Kind: Resource, Scope> {
    pub scope: Scope,
    pub dyn_type: Kind::DynamicType,
}
impl<Kind: Resource, Scope: scope::Scope> Verb for Watch<Kind, Scope> {
    type ResponseDecoder = DecodeStream<WatchEvent<Kind>>;

    fn to_http_request(&self) -> Result<Request<Body>> {
        Request::get(format!(
            "{}?watch=1",
            Kind::url_path(&self.dyn_type, self.scope.namespace()),
        ))
        .body(Body::empty())
        .context(BuildRequestFailed)
    }
}
