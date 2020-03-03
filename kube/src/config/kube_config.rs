use std::path::Path;

#[cfg(feature = "native-tls")]
use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};

use reqwest::{Certificate, Identity};

use crate::{
    config::apis::{AuthInfo, Cluster, Config, Context},
    Error, Result,
};

/// Regardless of tls type, a Certificate Der is always a byte array
pub struct Der(pub Vec<u8>);

use std::convert::TryFrom;
impl TryFrom<Der> for Certificate {
    type Error = Error;

    fn try_from(val: Der) -> Result<Certificate> {
        Certificate::from_der(&val.0).map_err(Error::ReqwestError)
    }
}

/// KubeConfigLoader loads current context, cluster, and authentication information.
#[derive(Clone, Debug)]
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
        let config = Config::read_from(path)?;
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

    #[cfg(feature = "native-tls")]
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

    #[cfg(feature = "rustls-tls")]
    pub fn identity(&self, _password: &str) -> Result<Identity> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;

        let mut buffer = client_key.clone();
        buffer.extend_from_slice(client_cert);
        Identity::from_pem(&buffer.as_slice()).map_err(|e| Error::SslError(format!("{}", e)))
    }

    #[cfg(feature = "native-tls")]
    pub fn ca_bundle(&self) -> Result<Option<Vec<Der>>> {
        let bundle = self
            .cluster
            .load_certificate_authority()
            .map_err(|e| Error::SslError(format!("{}", e)))?;

        if bundle.is_none() {
            return Ok(None)
        }
        let bundle = bundle.unwrap();

        let bundle = X509::stack_from_pem(&bundle).map_err(|e| Error::SslError(format!("{}", e)))?;

        let mut stack = vec![];
        for ca in bundle {
            let der = ca.to_der().map_err(|e| Error::SslError(format!("{}", e)))?;
            stack.push(Der(der))
        }
        Ok(Some(stack))
    }

    #[cfg(feature = "rustls-tls")]
    pub fn ca_bundle(&self) -> Result<Option<Vec<Der>>> {
        use rustls::internal::pemfile;
        use std::io::Cursor;
        let bundle = self.cluster.load_certificate_authority()?;

        if bundle.is_none() {
            return Ok(None)
        }
        let bundle = bundle.unwrap();

        let mut pem = Cursor::new(bundle);
        pem.set_position(0);

        let mut stack = vec![];
        for ca in pemfile::certs(&mut pem).map_err(|e| Error::SslError(format!("{:?}", e)))? {
            stack.push(Der(ca.0))
        }
        Ok(Some(stack))
    }
}
