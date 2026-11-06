#[cfg(any(feature = "http-proxy", feature = "socks5"))]
use crate::config::{NoProxyError, no_proxy::NoProxy};

use crate::{
    config::{
        KubeconfigError,
        file_config::{AuthInfo, Cluster, Context, Kubeconfig},
    },
    util::nonempty,
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

    fn load_from_kubeconfig(
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
            .and_then(|named_context| named_context.context.clone())
            .ok_or_else(|| KubeconfigError::LoadContext(context_name.clone()))?;

        let cluster_name = cluster.unwrap_or(&current_context.cluster);
        let cluster = config
            .clusters
            .iter()
            .find(|named_cluster| &named_cluster.name == cluster_name)
            .and_then(|named_cluster| named_cluster.cluster.clone())
            .ok_or_else(|| KubeconfigError::LoadClusterOfContext(cluster_name.clone()))?;

        let user_name = user.or_else(|| current_context.user.as_ref());

        // client-go doesn't fail on empty/missing user, so we don't either
        // see https://github.com/kube-rs/kube/issues/1594
        let mut auth_info = if let Some(user) = user_name {
            config
                .auth_infos
                .iter()
                .find(|named_user| &named_user.name == user)
                .and_then(|named_user| named_user.auth_info.clone())
                .unwrap_or_else(AuthInfo::default)
        } else {
            AuthInfo::default()
        };

        if let Some(exec_config) = &mut auth_info.exec
            && exec_config.provide_cluster_info
        {
            exec_config.cluster = Some((&cluster).try_into()?);
        }

        Ok(ConfigLoader {
            current_context,
            cluster,
            user: auth_info,
        })
    }

    pub async fn load(
        config: Kubeconfig,
        context: Option<&String>,
        cluster: Option<&String>,
        user: Option<&String>,
    ) -> Result<Self, KubeconfigError> {
        Self::load_from_kubeconfig(config, context, cluster, user)
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

    /// The proxy to reach the apiserver at `cluster_url` through, if any.
    pub fn proxy_url(&self, cluster_url: &http::Uri) -> Result<Option<http::Uri>, KubeconfigError> {
        let proxy = nonempty(self.cluster.proxy_url.clone())
            .or_else(|| nonempty(std::env::var("HTTPS_PROXY").ok()))
            .or_else(|| nonempty(std::env::var("https_proxy").ok()))
            .map(|proxy| proxy.parse::<http::Uri>().map_err(KubeconfigError::ParseProxyUrl))
            .transpose()?;

        #[cfg(any(feature = "http-proxy", feature = "socks5"))]
        {
            Self::apply_no_proxy(proxy, cluster_url, NoProxy::from_env)
        }

        #[cfg(not(any(feature = "http-proxy", feature = "socks5")))]
        {
            let _ = cluster_url; // NO_PROXY is only honored with a proxy feature
            Ok(proxy)
        }
    }

    /// Drops `proxy` if `NO_PROXY` exempts `cluster_url`. Only reads `NO_PROXY` when a proxy is configured.
    #[cfg(any(feature = "http-proxy", feature = "socks5"))]
    fn apply_no_proxy(
        proxy: Option<http::Uri>,
        cluster_url: &http::Uri,
        load_no_proxy: impl FnOnce() -> Result<Option<NoProxy>, NoProxyError>,
    ) -> Result<Option<http::Uri>, KubeconfigError> {
        let Some(proxy) = proxy else {
            return Ok(None);
        };
        if let Some(no_proxy) = load_no_proxy().map_err(KubeconfigError::ParseNoProxy)?
            && no_proxy
                .matches(cluster_url)
                .map_err(KubeconfigError::ParseNoProxy)?
        {
            return Ok(None);
        }
        Ok(Some(proxy))
    }
}

impl TryFrom<Kubeconfig> for ConfigLoader {
    type Error = KubeconfigError;

    fn try_from(kubeconfig: Kubeconfig) -> Result<Self, KubeconfigError> {
        Self::load_from_kubeconfig(kubeconfig, None, None, None)
    }
}

#[cfg(all(test, any(feature = "http-proxy", feature = "socks5")))]
mod tests {
    use rstest::rstest;

    use super::*;

    // NO_PROXY is matched against the apiserver, never against the proxy itself
    #[rstest]
    #[case("10.2.2.5", false)]
    #[case("10.0.0.0/8", false)]
    #[case("localhost, 10.2.2.5", false)]
    #[case("proxy.corporate.com", true)]
    #[case("proxy.corporate.com:80", true)]
    #[case("1.2.3.4", true)]
    fn no_proxy_matches_cluster_url(#[case] no_proxy: &str, #[case] proxied: bool) {
        let proxy_url = http::Uri::from_static("http://proxy.corporate.com:80");
        let cluster_url = http::Uri::from_static("https://10.2.2.5:6443");

        let proxy = ConfigLoader::apply_no_proxy(Some(proxy_url.clone()), &cluster_url, || {
            no_proxy.parse::<NoProxy>().map(Some)
        })
        .expect("valid proxy and NO_PROXY configuration");
        assert_eq!(proxy, proxied.then_some(proxy_url), "NO_PROXY={no_proxy}");
    }

    #[test]
    fn no_proxy_is_not_read_without_proxy() {
        let cluster_url = http::Uri::from_static("https://kube.rs");
        let proxy = ConfigLoader::apply_no_proxy(None, &cluster_url, || {
            panic!("NO_PROXY must not be read without a proxy")
        })
        .expect("no proxy requires no NO_PROXY configuration");
        assert_eq!(proxy, None);
    }

    #[test]
    fn absent_no_proxy_preserves_proxy() {
        let proxy_url = http::Uri::from_static("http://proxy.corporate.com:80");
        let cluster_url = http::Uri::from_static("https://kube.rs");
        let proxy = ConfigLoader::apply_no_proxy(Some(proxy_url.clone()), &cluster_url, || Ok(None))
            .expect("absent NO_PROXY is valid");
        assert_eq!(proxy, Some(proxy_url));
    }

    #[test]
    fn malformed_no_proxy_is_reported_with_proxy() {
        let proxy_url = http::Uri::from_static("http://proxy.corporate.com:80");
        let cluster_url = http::Uri::from_static("https://kube.rs");
        let result =
            ConfigLoader::apply_no_proxy(Some(proxy_url), &cluster_url, || ",".parse::<NoProxy>().map(Some));
        assert!(matches!(result, Err(KubeconfigError::ParseNoProxy(_))));
    }

    const PROXY: &str = "http://proxy.corporate.com:80";
    const CHILD_KUBECONFIG_PROXY: &str = "KUBE_TEST_PROXY_URL_KUBECONFIG";
    const CHILD_EXPECT: &str = "KUBE_TEST_PROXY_URL_EXPECT";

    // The env can only be set safely for a child process, so this reruns `config_proxy_url_child` in one
    #[rstest]
    #[case(Some(PROXY), None, None, true)]
    #[case(Some(PROXY), None, Some("10.2.2.5"), false)]
    #[case(Some(PROXY), None, Some("proxy.corporate.com:80"), true)]
    #[case(Some(PROXY), None, Some("*"), false)]
    #[case(None, Some(PROXY), None, true)]
    #[case(None, Some(PROXY), Some("10.2.2.5"), false)]
    fn config_proxy_url(
        #[case] kubeconfig_proxy: Option<&str>,
        #[case] https_proxy: Option<&str>,
        #[case] no_proxy: Option<&str>,
        #[case] proxied: bool,
    ) {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child.args(["--exact", "config::file_loader::tests::config_proxy_url_child"]);
        for var in [
            "HTTPS_PROXY",
            "https_proxy",
            "NO_PROXY",
            "no_proxy",
            CHILD_KUBECONFIG_PROXY,
        ] {
            child.env_remove(var);
        }
        if let Some(proxy) = kubeconfig_proxy {
            child.env(CHILD_KUBECONFIG_PROXY, proxy);
        }
        if let Some(proxy) = https_proxy {
            child.env("HTTPS_PROXY", proxy);
        }
        if let Some(no_proxy) = no_proxy {
            child.env("NO_PROXY", no_proxy);
        }
        child.env(CHILD_EXPECT, if proxied { PROXY } else { "" });
        let output = child.output().unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success() && stdout.contains("1 passed"), "{stdout}");
    }

    // Does nothing unless started by `config_proxy_url`
    #[tokio::test]
    async fn config_proxy_url_child() {
        let Ok(expect) = std::env::var(CHILD_EXPECT) else {
            return;
        };
        let proxy_line = std::env::var(CHILD_KUBECONFIG_PROXY)
            .map(|proxy| format!("    proxy-url: {proxy}\n"))
            .unwrap_or_default();
        let kubeconfig = Kubeconfig::from_yaml(&format!(
            "clusters:\n- name: c\n  cluster:\n    server: https://10.2.2.5:6443\n{proxy_line}\
             contexts:\n- name: c\n  context: {{ cluster: c, user: u }}\n\
             users:\n- name: u\n  user: {{}}\ncurrent-context: c\n"
        ))
        .unwrap();
        let config = crate::Config::from_custom_kubeconfig(kubeconfig, &Default::default())
            .await
            .unwrap();
        assert_eq!(
            config.proxy_url,
            nonempty(Some(expect)).map(|p| p.parse().unwrap())
        );
    }
}
