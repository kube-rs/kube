use std::path::Path;

#[cfg(feature = "native-tls")]
use openssl::{
    pkcs12::Pkcs12,
    pkey::PKey,
    x509::X509,
};
use crate::{Error, Result};
use crate::config::apis::{AuthInfo, Cluster, Config, Context};

/// KubeConfigLoader loads current context, cluster, and authentication information.
#[derive(Clone,Debug)]
pub struct KubeConfigLoader {
    pub current_context: Context,
    pub cluster: Cluster,
    pub user: AuthInfo,
}

impl KubeConfigLoader {
    pub async fn load<P: AsRef<Path>>(
        path: P,
        context: Option<String>,
        cluster: Option<String>,
        user: Option<String>,
    ) -> Result<KubeConfigLoader> {
        let config = Config::load_config(path)?;
        let context_name = context.as_ref().unwrap_or(&config.current_context);
        let current_context = config
            .contexts
            .iter()
            .find(|named_context| &named_context.name == context_name)
            .map(|named_context| &named_context.context)
            .ok_or_else(|| Error::KubeConfig("Unable to load current context".into()))?;
        let cluster_name = cluster.as_ref().unwrap_or(&current_context.cluster);
        let cluster = config
            .clusters
            .iter()
            .find(|named_cluster| &named_cluster.name == cluster_name)
            .map(|named_cluster| &named_cluster.cluster)
            .ok_or_else(|| Error::KubeConfig("Unable to load cluster of context".into()))?;
        let user_name = user.as_ref().unwrap_or(&current_context.user);

        // NB: can't have async closures yet
        let mut user_opt = None;
        for named_user in config.auth_infos {
            if &named_user.name == user_name {
                let mut user = named_user.auth_info.clone();
                user.load_gcp().await?;
                user_opt = Some(user);
            }
        }
        let user = user_opt.ok_or_else(|| Error::KubeConfig("Unable to find named user".into()))?;
        Ok(KubeConfigLoader {
            current_context: current_context.clone(),
            cluster: cluster.clone(),
            user,
        })
    }

    pub fn p12(&self, password: &str) -> Result<Pkcs12> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let x509 = X509::from_pem(&client_cert).map_err(|e| Error::SslError(format!("{}", e)))?;
        let pkey = PKey::private_key_from_pem(&client_key).map_err(|e| Error::SslError(format!("{}", e)))?;

        Pkcs12::builder()
            .build(password, "kubeconfig", &pkey, &x509)
            .map_err(|e| Error::SslError(format!("{}", e)))
    }

    pub fn ca_bundle(&self) -> Option<Result<Vec<X509>>> {
        let bundle = self.cluster.load_certificate_authority().ok()?;
        Some(X509::stack_from_pem(&bundle).map_err(|e| Error::SslError(format!("{}", e))))
    }
}
