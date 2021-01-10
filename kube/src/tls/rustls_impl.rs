use crate::{config::Der, Error, Result};
use reqwest::Identity;
use rustls::{
    internal::pemfile,
    sign::{RSASigningKey, SigningKey},
};
use std::io::Cursor;

pub fn sign(signature_base: &str, private_key: &str) -> Result<Vec<u8>> {
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

pub fn ca_bundle(bundle: &[u8]) -> Result<Vec<Der>> {
    let mut pem = Cursor::new(bundle);
    pem.set_position(0);

    let mut stack = vec![];
    for ca in pemfile::certs(&mut pem).map_err(|e| Error::SslError(format!("{:?}", e)))? {
        stack.push(Der(ca.0))
    }
    return Ok(stack);
}

pub fn identity(_password: &str, client_cert: &[u8], client_key: &[u8]) -> Result<Vec<u8>> {
    let mut buffer = client_key.to_vec();
    buffer.extend_from_slice(client_cert);
    // Make sure the buffer can be parsed properly but throw away the result
    let _identity = Identity::from_pem(&buffer.as_slice())?;
    Ok(buffer)
}
