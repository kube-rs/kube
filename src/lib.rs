#[macro_use]
extern crate failure;
#[macro_use]
extern crate serde_derive;

extern crate base64;
extern crate dirs;
extern crate k8s_openapi;
extern crate openssl;
extern crate reqwest;
extern crate serde;
extern crate serde_yaml;

pub mod client;
pub mod config;
