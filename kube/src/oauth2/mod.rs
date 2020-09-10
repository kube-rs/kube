use std::{
    env,
    fs::File,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[cfg(feature = "rustls-tls")] use crate::error::Error;
use crate::{error::ConfigError, Result};

use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use url::form_urlencoded::Serializer;

const GOOGLE_APPLICATION_CREDENTIALS: &str = "GOOGLE_APPLICATION_CREDENTIALS";
const DEFAULT_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:jwt-bearer";

#[derive(Debug, Serialize)]
struct Header {
    alg: String,
    typ: String,
}

// https://github.com/golang/oauth2/blob/c85d3e98c914e3a33234ad863dcbff5dbc425bb8/jws/jws.go#L34-L52
#[derive(Debug, Serialize)]
struct Claim {
    iss: String,
    scope: String,
    aud: String,
    exp: u64,
    iat: u64,
}

impl Claim {
    fn new(c: &Credentials, scope: &[String]) -> Claim {
        let iat = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time is before UNIX_EPOCH");

        // The access token is available for 1 hour.
        // https://github.com/golang/oauth2/blob/c85d3e98c914e3a33234ad863dcbff5dbc425bb8/jws/jws.go#L63
        let exp = iat + Duration::from_secs(3600);

        Claim {
            iss: c.client_email.clone(),
            scope: scope.join(" "),
            aud: c.token_uri.clone(),
            exp: exp.as_secs(),
            iat: iat.as_secs(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    #[serde(rename = "type")]
    typ: String,
    project_id: String,
    private_key_id: String,
    private_key: String,
    client_email: String,
    client_id: String,
    auth_uri: String,
    token_uri: String,
    auth_provider_x509_cert_url: String,
    client_x509_cert_url: String,
}

impl Credentials {
    pub fn load() -> Result<Credentials> {
        let path = env::var_os(GOOGLE_APPLICATION_CREDENTIALS)
            .map(PathBuf::from)
            .ok_or(ConfigError::MissingGoogleCredentials)?;
        let f = File::open(path).map_err(ConfigError::OAuth2LoadCredentials)?;
        let config = serde_json::from_reader(f).map_err(ConfigError::OAuth2ParseCredentials)?;
        Ok(config)
    }
}

pub struct CredentialsClient {
    pub credentials: Credentials,
    pub client: reqwest::Client,
}

// https://github.com/golang/oauth2/blob/c85d3e98c914e3a33234ad863dcbff5dbc425bb8/internal/token.go#L61-L66
#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<i64>,
}

impl TokenResponse {
    pub fn into_token(self) -> Token {
        Token {
            access_token: self.access_token.unwrap(),
            token_type: self.token_type.unwrap(),
            refresh_token: String::new(),
            expiry: self.expires_in,
        }
    }
}

// https://github.com/golang/oauth2/blob/c85d3e98c914e3a33234ad863dcbff5dbc425bb8/token.go#L31-L55
#[derive(Debug)]
pub struct Token {
    pub access_token: String,
    pub token_type: String,
    pub refresh_token: String,
    pub expiry: Option<i64>,
}

impl CredentialsClient {
    pub fn new() -> Result<CredentialsClient> {
        Ok(CredentialsClient {
            credentials: Credentials::load()?,
            client: reqwest::Client::new(),
        })
    }

    pub async fn request_token(&self, scopes: &[String]) -> Result<Token> {
        let encoded = jws_encode(
            &Claim::new(&self.credentials, scopes),
            &Header {
                alg: "RS256".to_string(),
                typ: "JWT".to_string(),
            },
            &self.credentials.private_key,
        )?;

        let body = Serializer::new(String::new())
            .extend_pairs(vec![
                ("grant_type".to_string(), DEFAULT_GRANT_TYPE.to_string()),
                ("assertion".to_string(), encoded.to_string()),
            ])
            .finish();

        let token_response = self
            .client
            .post(&self.credentials.token_uri)
            .body(body)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .send()
            .await
            .map_err(ConfigError::OAuth2RequestToken)
            .and_then(|response| {
                if response.status() != reqwest::StatusCode::OK {
                    Err(ConfigError::OAuth2RetrieveCredentials(Box::new(response)))
                } else {
                    Ok(response)
                }
            })?
            .json::<TokenResponse>()
            .await
            .map_err(ConfigError::OAuth2ParseToken)?;
        Ok(token_response.into_token())
    }
}

fn jws_encode(claim: &Claim, header: &Header, private_key: &str) -> Result<String> {
    let encoded_header = base64_encode(serde_json::to_string(&header).unwrap().as_bytes());
    let encoded_claims = base64_encode(serde_json::to_string(&claim).unwrap().as_bytes());
    let signature_base = format!("{}.{}", encoded_header, encoded_claims);
    let signature = sign(&signature_base, private_key)?;
    let encoded_signature = base64_encode(&signature);
    Ok(format!("{}.{}", signature_base, encoded_signature))
}

#[cfg(feature = "native-tls")]
fn sign(signature_base: &str, private_key: &str) -> Result<Vec<u8>> {
    use openssl::{hash::MessageDigest, pkey::PKey, rsa::Padding, sign::Signer};
    let key = PKey::private_key_from_pem(private_key.as_bytes())?;
    let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
    signer.set_rsa_padding(Padding::PKCS1)?;
    signer.update(signature_base.as_bytes())?;
    Ok(signer.sign_to_vec()?)
}

#[cfg(feature = "rustls-tls")]
fn sign(signature_base: &str, private_key: &str) -> Result<Vec<u8>> {
    use rustls::{
        internal::pemfile,
        sign::{RSASigningKey, SigningKey},
    };

    let keys = pemfile::pkcs8_private_keys(&mut private_key.as_bytes())
        .map_err(|_| Error::SslError("fail to parse private key".into()))?;
    let key = keys
        .get(0)
        .ok_or_else(|| Error::SslError("no usable private key found to sign with RS256".into()))?;
    let signing_key =
        RSASigningKey::new(key).map_err(|_| Error::SslError("fail to make RSA signing key".into()))?;
    let signer = signing_key
        .choose_scheme(&[rustls::SignatureScheme::RSA_PKCS1_SHA256])
        .ok_or_else(|| Error::SslError("scheme RSA_PKCS1_SHA256 not found into private key".into()))?;
    signer
        .sign(signature_base.as_bytes())
        .map_err(|e| Error::SslError(format!("{}", e)))
}

fn base64_encode(bytes: &[u8]) -> String {
    base64::encode_config(bytes, base64::URL_SAFE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_true() {
        // generated with
        // ```
        // openssl genpkey -out rsakey.pem -algorithm RSA -pkeyopt rsa_keygen_bits:2048
        // ```
        let private_key = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDjT1UyWwk/v2UG
BhTEB+8NIL4RW3+u7TSOVP0Qpxf22bhJH9+RqwZPlwzbhYQT1TNXT4fFnARaGWaG
EtPV/rlV6o9PcLCMj3y2sOiKBy0qS6/3nYHKlFNPGnesYLTIbk54Orp4OYnSqQ/G
zBZbS3IRDsTaOb4D+KaxdPm/I8qN1TEPIDEkDRYtRprbmTQaz3rl0ooKuDCHiWoW
I7rG6zGkcGwBZAkwh0XFeklJSwZbC0JK88wolHKWJba6KCO8A2LpskacPB/KP5mQ
bnTzIS5xiNMKf9qhLm/HgDzgCL9E8StnZygUmRFKYh4MTzrpfGaoT5Vm+tijlrDi
CDE33tuZAgMBAAECggEBANuVDnsngCbJsECCbVr1QxNOdu1zk0ObN3LrXM/Sao72
wVQ6axFfwifuhegl8XHrOb51QHY/geC7utN3qpWFjOoXPbuC47nU/qfI+8oippm+
Jc2wpOnaISRAMC0f+mPIUxtHuExdYOtUj7399vbYSed6eeVJdGqHsBerJXtkis44
uuzlQ6ISMPd3YhxN6S4QbPyw6aaoJG0qYpdHSL/n9r49hA3sKbAQVSOTzM1PMRje
6kB6BPrfmyVavHUXRZG1lU7gD41F8nG0wXOvsFu1XXPeEjw2/uBRanA8rWtAPv02
vBXcBMHpv7ySWCVOXMLWfZmo4GJIjfhtjTasUTSxVAECgYEA+Tvei02NBGWdjOzu
xoLvF71oT22Ic862StvmDJYSV/9bs1r8pxGCPs0tkHr/DQoBcmvAComrcEBltkaZ
yyKxDdSLrsy1nkL4hPMwJF0gNZAAaj4rMNfadKGOmlhQBcFCBph8kijcGsyYn1Vs
2cGCIZCALofDm4t8oIUpB8+UsYECgYEA6XsYh50+JkpuTknTbciOQJijl0a0oK6X
SO9mwoWEtg0+mrR3gL0kjghUObN5p0lhLLyqCBDVeFdaDcWbZXdF/KuSyI48Bube
c0EYkCFk1W/39yVb6LqQP6xoPrA/nLB4AuZYSqkZwx+bHH7OBgHaXRh2m2HawU5B
pQsM2PVwhhkCgYAonJfTzSw4VjKI/yadVEKPdL6liqycakeMBS8ESAPvMN4JaL8Y
niLCBv7wtwoOXt4DfglJ7krwPJ4WSITQ8/Mz1Ll6H0NM6Y7DYzkqA76226MlrMGu
8M1ZCeZJwjAv7+DJYFmUG3JaL5KDDBFznjONMpWgf2DhXKZPJcOc0TdigQKBgGHL
4NN1JsItLRT30WrLteISzXsg76naV54CQR27hYIn/BAbBW9USop/rJ/asFtE3kI5
6FKmknPsyti368ZNdnBGgZ4mDbiqXYUTQDGm+zB3zPqlmGDcPG2fTq7rbkm4lRxJ
1bO4LwVPKM5/wtY7UnbqN0wQaevMVqzF+ySpce+JAoGBAOLkdZrv7+JPfusIlj/H
CuNhikh1WMHm6Al0SYBURhUb52hHGEAnpaIxwQrN4PPD4Iboyp0kLsjMtKxlvnBm
WpsqFXdkj9GLZt1s1Q/5SW5Twb7gxdR7cXrXOcATivN1/GDdhIHS1NEb3MM7EBXc
9RSM375nLWCP0LDosgKSaq+u
-----END PRIVATE KEY-----
"#;
        let msg = "foo bar";
        let expected = "h0H_U6SO_i1F7JwzzTBHL1DNTw-YD6jdMul9Uwo_5xB_TmztP9c7T8e5CVY1o5_vMfQ3SZJXZ9liwd7FK8a7NjNumWIWq0_KZvDxMK6D2SSkA8WAz4KsdUU1CNVxM51UYQgYnHpaJvtNmowgzCnahNQQso4hKsYCe7nNKlTiCP1yPzM4MWJYh2cekH1SGqSaOtgvQZz4GrOPG-hhcyMZMk_u-sZ0F3PUFj0-kfbhZPNVpvv4-wI_XA84q85Wech4nsgLbxO9397-whsmGVNlqqo2PwwxASn7dEqtrrvD7mkabf32OqmgJ-xXT_n4m67kvgzC7ausezX7E0zcnBj3RQ==".to_string();
        assert_eq!(base64_encode(&sign(&msg, private_key).unwrap()), expected);
    }
}
