use std::path::Path;

use failure::Error;
use openssl::pkcs12::Pkcs12;
use openssl::pkey::PKey;
use openssl::x509::X509;

use crate::config::apis::{AuthInfo, Cluster, Config, Context};

/// KubeConfigLoader loads current context, cluster, and authentication information.
#[derive(Debug)]
pub struct KubeConfigLoader {
    pub current_context: Context,
    pub cluster: Cluster,
    pub user: AuthInfo,
}

impl KubeConfigLoader {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<KubeConfigLoader, Error> {
        let config = Config::load_config(path)?;
        let current_context = config
            .contexts
            .iter()
            .find(|named_context| named_context.name == config.current_context)
            .map(|named_context| &named_context.context)
            .ok_or(format_err!("Unable to load current context"))?;
        let cluster = config
            .clusters
            .iter()
            .find(|named_cluster| named_cluster.name == current_context.cluster)
            .map(|named_cluster| &named_cluster.cluster)
            .ok_or(format_err!("Unable to load cluster of current context"))?;
        let user = config
            .auth_infos
            .iter()
            .find(|named_user| named_user.name == current_context.user)
            .map(|named_user| {
                let mut user = named_user.auth_info.clone();
                match user.load_gcp() {
                    Ok(_) => Ok(user),
                    Err(e) => Err(e),
                }
            })
            .ok_or(format_err!("Unable to load user of current context"))??;
        Ok(KubeConfigLoader {
            current_context: current_context.clone(),
            cluster: cluster.clone(),
            user: user.clone(),
        })
    }

    pub fn p12(&self, password: &str) -> Result<Pkcs12, Error> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let x509 = X509::from_pem(&client_cert)?;
        let pkey = PKey::private_key_from_pem(&client_key)?;

        Pkcs12::builder()
            .build(password, "kubeconfig", &pkey, &x509)
            .map_err(Error::from)
    }

    pub fn ca(&self) -> Option<Result<X509, Error>> {
        let ca = self.cluster.load_certificate_authority()?;
        Some(ca.and_then(|ca| X509::from_pem(&ca).map_err(Error::from)))
    }
}
