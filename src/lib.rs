#[macro_use] extern crate failure;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate log;

pub mod client;
pub mod config;
pub mod api;
mod oauth2;

pub use failure::Error;
pub type Result<T> = std::result::Result<T, Error>;
