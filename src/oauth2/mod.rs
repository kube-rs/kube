use std::env;
use std::fs::File;
use std::path::PathBuf;

use chrono::Utc;
use failure::Error;
use openssl::pkey::{PKey, Private};
use openssl::sign::Signer;
use openssl::rsa::Padding;
use openssl::hash::MessageDigest;
use reqwest::Client;
use reqwest::header::CONTENT_TYPE;
use time::Duration;
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
    exp: i64,
    iat: i64,
}

impl Claim {
    fn new(c: &Credentials, scope: &Vec<String>) -> Claim {
        let iat = Utc::now();
        // The access token is available for 1 hour.
        // https://github.com/golang/oauth2/blob/c85d3e98c914e3a33234ad863dcbff5dbc425bb8/jws/jws.go#L63
        let exp = iat + Duration::hours(1);
        Claim {
            iss: c.client_email.clone(),
            scope: scope.join(" "),
            aud: c.token_uri.clone(),
            exp: exp.timestamp(),
            iat: iat.timestamp(),
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
    pub fn load() -> Result<Credentials, Error> {
        let path = env::var_os(GOOGLE_APPLICATION_CREDENTIALS)
            .map(PathBuf::from)
            .ok_or(format_err!(
                "Missing {} env",
                GOOGLE_APPLICATION_CREDENTIALS
            ))?;
        let f = File::open(path)?;
        let config = serde_json::from_reader(f)?;
        Ok(config)
    }
}

pub struct CredentialsClient {
    pub credentials: Credentials,
    pub client: Client,
}

// https://github.com/golang/oauth2/blob/c85d3e98c914e3a33234ad863dcbff5dbc425bb8/internal/token.go#L61-L66
#[derive(Debug, Serialize, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    token_type: Option<String>,
    expires_in: Option<i64>,
}

impl TokenResponse {
    pub fn to_token(self) -> Token {
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
    pub fn new() -> Result<CredentialsClient, Error> {
        Ok(CredentialsClient {
            credentials: Credentials::load()?,
            client: Client::new(),
        })
    }
    pub fn request_token(&self, scopes: &Vec<String>) -> Result<Token, Error> {
        let private_key = PKey::private_key_from_pem(&self.credentials.private_key.as_bytes())?;
        let encoded = &self.jws_encode(
            &Claim::new(&self.credentials, scopes),
            &Header{
                alg: "RS256".to_string(),
                typ: "JWT".to_string(),
            },
            private_key)?;

        let body = Serializer::new(String::new())
            .extend_pairs(vec![
                ("grant_type".to_string(), DEFAULT_GRANT_TYPE.to_string()),
                ("assertion".to_string(), encoded.to_string()),
            ]).finish();
        let token_response: TokenResponse = self.client
            .post(&self.credentials.token_uri)
            .body(body)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .send()?
            .json()?;
        Ok(token_response.to_token())
    }

    fn jws_encode(&self, claim: &Claim, header: &Header, key: PKey<Private>) -> Result<String, Error> {
        let encoded_header = self.base64_encode(serde_json::to_string(&header).unwrap().as_bytes());
        let encoded_claims = self.base64_encode(serde_json::to_string(&claim).unwrap().as_bytes());
        let signature_base = format!("{}.{}", encoded_header, encoded_claims);
        let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
        signer.set_rsa_padding(Padding::PKCS1)?;
        signer.update(signature_base.as_bytes())?;
        let signature = signer.sign_to_vec()?;
        Ok(format!("{}.{}", signature_base, self.base64_encode(&signature)))
    }

    fn base64_encode(&self, bytes: &[u8]) -> String {
        base64::encode_config(bytes, base64::URL_SAFE)
    }
}
