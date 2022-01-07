use anyhow::Context;
use k8s_openapi::api::{
    authentication::v1::{TokenRequest, TokenRequestSpec},
    authorization::v1::{NonResourceAttributes, ResourceAttributes},
    core::v1::{ConfigMap, Pod, ServiceAccount},
};
use kube::{
    core::Request,
    discovery::verbs,
    util::auth::{
        AccessStatus, AuthClient, NonResourceAttributesBuilder, ResourceAttributesBuilder,
        SubjectAccessReviewBuilder, TokenValidity,
    },
    Resource,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = kube::Client::try_default().await?;
    let token = get_token(&client).await?;
    let client = AuthClient::new(client);
    let attrs = ResourceAttributesBuilder::new()
        .set_from_resource::<Pod>(&())
        .verb(verbs::CREATE)
        .all_objects()
        .build();
    check_has_access("create pods", &token, Attrs::Resource(attrs), &client).await?;
    let attrs = ResourceAttributesBuilder::new()
        .set_from_resource::<ConfigMap>(&())
        .verb(verbs::GET)
        .object_name("kube-root-ca.crt")
        .namespace("default")
        .build();
    check_has_access(
        "discover apiserver TLS sertificate trust chain",
        &token,
        Attrs::Resource(attrs),
        &client,
    )
    .await?;
    let attrs = NonResourceAttributesBuilder::new()
        .verb("GET")
        .path("/metrics")
        .build();
    check_has_access(
        "get apiserver metrics",
        &token,
        Attrs::NonResource(attrs),
        &client,
    )
    .await?;

    Ok(())
}

async fn get_token(client: &kube::Client) -> anyhow::Result<String> {
    if let Ok(t) = std::env::var("TOKEN") {
        println!("Using token from the TOKEN environment variable");
        return Ok(t);
    }
    if let (Ok(ns), Ok(sa)) = (std::env::var("NAMESPACE"), std::env::var("SERVICE_ACCOUNT")) {
        let audience = std::env::var("AUDIENCE").ok();
        println!(
            "Requesting token for service account {} in namespace {}, intended for audience {:?}",
            ns, sa, audience
        );
        let token_request_data = TokenRequest {
            spec: TokenRequestSpec {
                audiences: audience.into_iter().collect(),
                expiration_seconds: Some(600),
                ..Default::default()
            },
            ..Default::default()
        };
        let token_request_data = serde_json::to_vec(&token_request_data)?;

        let token_request = Request {
            url_path: ServiceAccount::url_path(&(), Some(&ns)),
        };
        let token_request =
            token_request.create_subresource("token", &sa, &Default::default(), token_request_data)?;
        let token_request: TokenRequest = client.request(token_request).await?;
        let token = token_request
            .status
            .map(|st| st.token)
            .context("token not returned")?;
        return Ok(token);
    }
    anyhow::bail!("Token not specified");
}

enum Attrs {
    Resource(ResourceAttributes),
    NonResource(NonResourceAttributes),
}

async fn check_has_access(
    description: &str,
    token: &str,
    attrs: Attrs,
    client: &AuthClient,
) -> anyhow::Result<()> {
    let user_info = match client.validate_token(token, &[]).await? {
        TokenValidity::Valid { user, .. } => user,
        TokenValidity::Invalid { reason } => {
            println!("Token is not valid: {:?}", reason);
            return Ok(());
        }
        TokenValidity::Fail => {
            println!("Internal error");
            return Ok(());
        }
    };

    let mut spec_builder = SubjectAccessReviewBuilder::new();
    spec_builder.set_from_user_info(user_info);

    let spec = match attrs {
        Attrs::Resource(res) => spec_builder.resource(res),
        Attrs::NonResource(nr) => spec_builder.non_resource(nr),
    };

    let status = client.check_access(spec).await?;

    print!("{}: ", description);
    match status {
        AccessStatus::Allow => {
            println!("allowed");
        }
        AccessStatus::Deny { reason } => {
            println!("denied: {:?}", reason);
        }
        AccessStatus::NoOpinion => {
            println!("neither allowed nor denied (no opinion)");
        }
        AccessStatus::Fail => {
            println!("internal error");
        }
    }

    Ok(())
}
