use super::{
    file_config::{AuthInfo, Cluster, Context, Kubeconfig},
    KubeconfigError,
};

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
    pub async fn new_from_options(options: &KubeConfigOptions) -> Result<Self, KubeconfigError> {
        let config = Kubeconfig::read()?;
        let loader = Self::load(
            config,
            options.context.as_ref(),
            options.cluster.as_ref(),
            options.user.as_ref(),
        )
        .await?;

        Ok(loader)
    }

    pub async fn new_from_kubeconfig(
        config: Kubeconfig,
        options: &KubeConfigOptions,
    ) -> Result<Self, KubeconfigError> {
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
    ) -> Result<Self, KubeconfigError> {
        let context_name = if let Some(name) = context {
            name
        } else if let Some(name) = &config.current_context {
            name
        } else {
            return Err(KubeconfigError::CurrentContextNotSet);
        };
        let current_context = config
            .contexts
            .iter()
            .find(|named_context| &named_context.name == context_name)
            .map(|named_context| &named_context.context)
            .ok_or_else(|| KubeconfigError::LoadContext(context_name.clone()))?;

        let cluster_name = cluster.unwrap_or(&current_context.cluster);
        let cluster = config
            .clusters
            .iter()
            .find(|named_cluster| &named_cluster.name == cluster_name)
            .map(|named_cluster| &named_cluster.cluster)
            .ok_or_else(|| KubeconfigError::LoadClusterOfContext(cluster_name.clone()))?;

        let user_name = user.unwrap_or(&current_context.user);
        let user = config
            .auth_infos
            .iter()
            .find(|named_user| &named_user.name == user_name)
            .map(|named_user| &named_user.auth_info)
            .ok_or_else(|| KubeconfigError::FindUser(user_name.clone()))?;

        Ok(ConfigLoader {
            current_context: current_context.clone(),
            cluster: cluster.clone(),
            user: user.clone(),
        })
    }

    pub fn ca_bundle(&self) -> Result<Option<Vec<Vec<u8>>>, KubeconfigError> {
        if let Some(bundle) = self.cluster.load_certificate_authority()? {
            Ok(Some(
                super::certs(&bundle).map_err(KubeconfigError::ParseCertificates)?,
            ))
        } else {
            Ok(None)
        }
    }

    pub fn proxy_url(&self) -> Result<Option<http::Uri>, KubeconfigError> {
        let nonempty = |o: Option<String>| o.filter(|s| !s.is_empty());

        if let Some(proxy) = nonempty(self.cluster.proxy_url.clone())
            .or_else(|| nonempty(std::env::var("HTTP_PROXY").ok()))
            .or_else(|| nonempty(std::env::var("http_proxy").ok()))
            .or_else(|| nonempty(std::env::var("HTTPS_PROXY").ok()))
            .or_else(|| nonempty(std::env::var("https_proxy").ok()))
        {
            Ok(Some(
                proxy
                    .parse::<http::Uri>()
                    .map_err(KubeconfigError::ParseProxyUrl)?,
            ))
        } else {
            Ok(None)
        }
    }
}
