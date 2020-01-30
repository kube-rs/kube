use std::path::Path;

#[cfg(feature = "native-tls")]
use openssl::{
    pkcs12::Pkcs12,
    pkey::PKey,
    x509::X509,
};

use reqwest::{Identity, Certificate};

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

    #[cfg(feature="native-tls")]
    pub fn identity(&self, password: &str) -> Result<Identity> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let x509 = X509::from_pem(&client_cert).map_err(|e| Error::SslError(format!("{}", e)))?;
        let pkey = PKey::private_key_from_pem(&client_key).map_err(|e| Error::SslError(format!("{}", e)))?;

        let p12 = Pkcs12::builder()
            .build(password, "kubeconfig", &pkey, &x509)
            .map_err(|e| Error::SslError(format!("{}", e)))?;

        let der = p12.to_der().map_err(|e| Error::SslError(format!("{}", e)))?;
        Ok(Identity::from_pkcs12_der(&der, password)?)
    }

    #[cfg(feature="rustls-tls")]
    pub fn identity(&self, password: &str) -> Result<Identity> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let mut buffer = client_key.clone();
        buffer.extend(client_cert);

        let id = Identity::from_pem(buffer.as_slice())
            .map_err(|e| Error::SslError(format!("{}", e)))?;
        Ok(id)
    }

    #[cfg(feature="native-tls")]
    pub fn ca_bundle(&self) -> Result<Vec<Certificate>> {
        let bundle = self.cluster.load_certificate_authority()
            .map_err(|e| Error::SslError(format!("{}", e)))?;
        let bundle = X509::stack_from_pem(&bundle).map_err(|e| Error::SslError(format!("{}", e)))?;
        let mut cert_bundle = vec![];
        for ca in bundle {
            let der = ca.to_der().map_err(|e| Error::SslError(format!("{}", e)))?;
            let cert = Certificate::from_der(&der)
                .map_err(Error::ReqwestError)?;
            cert_bundle.push(cert);
        }
        Ok(cert_bundle)
    }

    #[cfg(feature = "rustls-tls")]
    pub fn ca_bundle(&self) -> Result<Vec<Certificate>> {
        let bundle = self.cluster.load_certificate_authority()?;
        let mut bundle_slice : &[u8] = &bundle;
        rustls::internal::pemfile::certs(&mut bundle_slice)
            .map_err(|e| Error::SslError(format!("{:?}", e)))?
            .into_iter()
            .map(|der| Certificate::from_der(&der.0)
                .map_err(|e| Error::SslError(format!("{:?}", e))))
            .collect()

    }
}

// HACKS
#[cfg(feature = "native-tls")]
pub fn will_catalina_fail_on_this_cert(_der: &Vec<u8>) -> bool {
    true
    //std::env::consts::OS == "macos" && der
    //    .not_before()
    //    .diff(der.not_after())
    //    .map(|d| d.days.abs() > 824)
    //    .unwrap_or(false)
}

#[cfg(feature = "rustls-tls")]
pub fn will_catalina_fail_on_this_cert(cert: &Certificate) -> bool { true }
