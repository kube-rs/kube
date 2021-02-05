use super::{
    auth,
    file_config::{AuthInfo, Cluster, Context, Kubeconfig},
    utils,
};
use crate::{error::ConfigError, Result};

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
        if let Some(named_user) = config.auth_infos.iter().find(|a| &a.name == user_name) {
            let mut user = named_user.auth_info.clone();
            if let Some(provider) = &user.auth_provider {
                user.token = auth::token_from_provider(provider).await?;
            }
            user_opt = Some(user);
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

    pub fn identity_pem(&self) -> Result<Vec<u8>> {
        let client_cert = &self.user.load_client_certificate()?;
        let client_key = &self.user.load_client_key()?;
        let mut buffer = client_key.clone();
        buffer.extend_from_slice(client_cert);
        Ok(buffer)
    }

    pub fn ca_bundle(&self) -> Result<Option<Vec<Vec<u8>>>> {
        if let Some(bundle) = self.cluster.load_certificate_authority()? {
            Ok(Some(utils::certs(&bundle)))
        } else {
            Ok(None)
        }
    }
}
