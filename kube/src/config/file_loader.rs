#[cfg(feature = "native-tls")]
use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
use reqwest::Identity;

use super::{
    file_config::{AuthInfo, Cluster, Context, Kubeconfig},
    utils, Der,
};
use crate::{error::ConfigError, Result};

#[cfg(feature = "rustls-tls")] use crate::Error;

/// KubeConfigOptions stores options used when loading kubeconfig file.
#[derive(Default, Clone)]
pub struct KubeConfigOptions {
    /// The named context to load
    pub context: Option<String>,
    /// The cluster to load
    pub cluster: Option<String>,
    /// The user to load
    pub user: Option<String>,
}

/// ConfigLoader loads current context, cluster, and authentication information
/// from a kubeconfig file.
#[derive(Clone, Debug)]
pub struct ConfigLoader {
    pub current_context: Context,
    pub cluster: Cluster,
    pub user: AuthInfo,
}

impl ConfigLoader {
    /// Returns a config loader based on the cluster information from the kubeconfig file.
    pub async fn new_from_options(options: &KubeConfigOptions) -> Result<Self> {
        let kubeconfig_path = utils::find_kubeconfig()
            .map_err(Box::new)
            .map_err(ConfigError::LoadConfigFile)?;

        let config = Kubeconfig::read_from(kubeconfig_path)?;
        let loader = Self::load(
            config,
            options.context.as_ref(),
            options.cluster.as_ref(),
            options.user.as_ref(),
        )
        .await?;

        Ok(loader)
    }

    pub async fn new_from_kubeconfig(config: Kubeconfig, options: &KubeConfigOptions) -> Result<Self> {
        let loader = Self::load(
            config,
            options.context.as_ref(),
            options.cluster.as_ref(),
            options.user.as_ref(),
        )
        .await?;

        Ok(loader)
    }

    pub async fn load(
        config: Kubeconfig,
        context: Option<&String>,
        cluster: Option<&String>,
        user: Option<&String>,
    ) -> Result<Self> {
        let context_name = context.unwrap_or(&config.current_context);
        let current_context = config
            .contexts
            .iter()
            .find(|named_context| &named_context.name == context_name)
            .map(|named_context| &named_context.context)
            .ok_or_else(|| ConfigError::LoadContext {
                context_name: context_name.clone(),
            })?;
        let cluster_name = cluster.unwrap_or(&current_context.cluster);
        let cluster = config
            .clusters
            .iter()
            .find(|named_cluster| &named_cluster.name == cluster_name)
            .map(|named_cluster| &named_cluster.cluster)
            .ok_or_else(|| ConfigError::LoadClusterOfContext {
                cluster_name: cluster_name.clone(),
            })?;
        let user_name = user.unwrap_or(&current_context.user);

        let mut user_opt = None;
        for named_user in config.auth_infos {
            if &named_user.name == user_name {
                let mut user = named_user.auth_info.clone();
                user.load_gcp().await?;
                user_opt = Some(user);
            }
        }
        let user = user_opt.ok_or_else(|| ConfigError::FindUser {
            user_name: user_name.clone(),
        })?;
        Ok(ConfigLoader {
            current_context: current_context.clone(),
            cluster: cluster.clone(),
            user,
        })
    }

    #[cfg(feature = "native-tls")]
    pub fn identity(&self, password: &str) -> Result<Vec<u8>> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let x509 = X509::from_pem(&client_cert)?;
        let pkey = PKey::private_key_from_pem(&client_key)?;

        let p12 = Pkcs12::builder().build(password, "kubeconfig", &pkey, &x509)?;

        let der = p12.to_der()?;
        // Make sure the buffer can be parsed properly but throw away the result
        let _identity = Identity::from_pkcs12_der(&der, password)?;
        Ok(der)
    }

    #[cfg(feature = "rustls-tls")]
    pub fn identity(&self, _password: &str) -> Result<Vec<u8>> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let mut buffer = client_key.clone();
        buffer.extend_from_slice(client_cert);
        // Make sure the buffer can be parsed properly but throw away the result
        let _identity = Identity::from_pem(&buffer.as_slice())?;
        Ok(buffer)
    }

    #[cfg(feature = "native-tls")]
    pub fn ca_bundle(&self) -> Result<Option<Vec<Der>>> {
        let bundle = self.cluster.load_certificate_authority()?;

        if let Some(bundle) = bundle {
            let bundle = X509::stack_from_pem(&bundle)?;

            let mut stack = vec![];
            for ca in bundle {
                let der = ca.to_der()?;
                stack.push(Der(der))
            }
            return Ok(Some(stack));
        }
        Ok(None)
    }

    #[cfg(feature = "rustls-tls")]
    pub fn ca_bundle(&self) -> Result<Option<Vec<Der>>> {
        use rustls::internal::pemfile;
        use std::io::Cursor;
        if let Some(bundle) = self.cluster.load_certificate_authority()? {
            let mut pem = Cursor::new(bundle);
            pem.set_position(0);

            let mut stack = vec![];
            for ca in pemfile::certs(&mut pem).map_err(|e| Error::SslError(format!("{:?}", e)))? {
                stack.push(Der(ca.0))
            }
            return Ok(Some(stack));
        }
        Ok(None)
    }
}
