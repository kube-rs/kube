//! Decode the result of a [`Verb`]
//!
//! You typically don't need to interact with this directly, unless you are implementing a custom [`Verb`]

pub mod single;
pub mod stream;

pub use single::DecodeSingle;
pub use stream::DecodeStream;
