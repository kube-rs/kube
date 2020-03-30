use crate::config::{self, kube_config::Der, ConfigLoader, ConfigOptions};
use crate::{Error, Result};

use reqwest::ClientBuilder;

/// Returns a client builder based on the cluster information from the kubeconfig file.
///
/// This allows to create your custom reqwest client for using with the cluster API.
pub async fn create(options: ConfigOptions) -> Result<ClientBuilder> {
    let loader = ConfigLoader::new_from_options(&options).await?;

    let token = match &loader.user.token {
        Some(token) => Some(token.clone()),
        None => {
            if let Some(exec) = &loader.user.exec {
                let creds = super::exec::auth_exec(exec)?;
                let status = creds.status.ok_or_else(|| {
                    Error::KubeConfig("exec-plugin response did not contain a status".into())
                })?;
                status.token
            } else {
                None
            }
        }
    };

    let mut client_builder = reqwest::Client::builder()
        // hard disallow more than 5 minute polls due to kubernetes limitations
        .timeout(std::time::Duration::new(295, 0));

    if let Some(ca_bundle) = loader.ca_bundle()? {
        use std::convert::TryInto;
        for ca in ca_bundle {
            client_builder = hacky_cert_lifetime_for_macos(client_builder, &ca);
            client_builder = client_builder.add_root_certificate(ca.try_into()?);
        }
    }

    match loader.identity(" ") {
        Ok(id) => {
            client_builder = client_builder.identity(id);
        }
        Err(e) => {
            debug!("failed to load client identity from kube config: {}", e);
            // last resort only if configs ask for it, and no client certs
            if let Some(true) = loader.cluster.insecure_skip_tls_verify {
                client_builder = client_builder.danger_accept_invalid_certs(true);
            }
        }
    }

    let mut headers = reqwest::header::HeaderMap::new();

    match (
        config::utils::data_or_file(&token, &loader.user.token_file),
        (&loader.user.username, &loader.user.password),
    ) {
        (Ok(token), _) => {
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", token))
                    .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
            );
        }
        (_, (Some(u), Some(p))) => {
            let encoded = base64::encode(&format!("{}:{}", u, p));
            headers.insert(
                reqwest::header::AUTHORIZATION,
                reqwest::header::HeaderValue::from_str(&format!("Basic {}", encoded))
                    .map_err(|e| Error::KubeConfig(format!("Invalid bearer token: {}", e)))?,
            );
        }
        _ => {}
    }

    Ok(client_builder.default_headers(headers))
}

// temporary catalina hack for openssl only
#[cfg(all(target_os = "macos", feature = "native-tls"))]
fn hacky_cert_lifetime_for_macos(client_builder: ClientBuilder, ca_: &Der) -> ClientBuilder {
    use openssl::x509::X509;
    let ca = X509::from_der(&ca_.0).expect("valid der is a der");
    if ca
        .not_before()
        .diff(ca.not_after())
        .map(|d| d.days.abs() > 824)
        .unwrap_or(false)
    {
        client_builder.danger_accept_invalid_certs(true)
    } else {
        client_builder
    }
}

#[cfg(any(not(target_os = "macos"), not(feature = "native-tls")))]
fn hacky_cert_lifetime_for_macos(client_builder: ClientBuilder, _: &Der) -> ClientBuilder {
    client_builder
}
