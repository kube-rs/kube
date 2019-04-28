#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;
extern crate log;

extern crate base64;
extern crate dirs;
extern crate openssl;
extern crate reqwest;
extern crate serde;
extern crate serde_yaml;
extern crate serde_json;
extern crate http;

pub mod client;
pub mod config;
pub mod api;

pub use failure::Error;
pub type Result<T> = std::result::Result<T, Error>;
