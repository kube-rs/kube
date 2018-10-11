mod apis;
mod loader;
mod utils;

use failure::Error;
use reqwest::{header, Certificate, Client, Identity};

use self::loader::KubeConfigLoader;

pub struct Configuration {
    pub base_path: String,
    pub client: Client,
}

impl Configuration {
    pub fn new(base_path: String, client: Client) -> Self {
        Configuration {
            base_path: base_path.to_owned(),
            client: client,
        }
    }
}

pub fn load_kube_config() -> Result<Configuration, Error> {
    let kubeconfig = utils::kube_path()
        .or_else(utils::default_kube_path)
        .ok_or(format_err!("Unable to load config"))?;

    let loader = KubeConfigLoader::load(kubeconfig)?;

    let password = " ";
    let p12 = loader.p12(password)?;
    let req_p12 = Identity::from_pkcs12_der(&p12.to_der()?, password)?;

    let ca = loader.ca()?;
    let req_ca = Certificate::from_der(&ca.to_der()?)?;

    let client_builder = Client::builder()
        .identity(req_p12)
        .add_root_certificate(req_ca);

    let mut headers = header::HeaderMap::new();
    headers.insert(header::AUTHORIZATION, header::HeaderValue::from_static(""));

    Ok(Configuration::new(
        loader.cluster.server,
        client_builder.build()?,
    ))
}
