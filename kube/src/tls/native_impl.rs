use openssl::{hash::MessageDigest, pkcs12::Pkcs12, pkey::PKey, rsa::Padding, sign::Signer, x509::X509};
use reqwest::Identity;

use crate::{config::Der, Result};

pub fn identity(password: &str, client_cert: &[u8], client_key: &[u8]) -> Result<Vec<u8>> {
    let x509 = X509::from_pem(&client_cert)?;
    let pkey = PKey::private_key_from_pem(&client_key)?;

    let p12 = Pkcs12::builder().build(password, "kubeconfig", &pkey, &x509)?;

    let der = p12.to_der()?;
    // Make sure the buffer can be parsed properly but throw away the result
    let _identity = Identity::from_pkcs12_der(&der, password)?;
    Ok(der)
}

pub fn ca_bundle(bundle: &[u8]) -> Result<Vec<Der>> {
    let bundle = X509::stack_from_pem(&bundle)?;

    let mut stack = vec![];
    for ca in bundle {
        let der = ca.to_der()?;
        stack.push(Der(der))
    }
    return Ok(stack);
}

pub fn sign(signature_base: &str, private_key: &str) -> Result<Vec<u8>> {
    let key = PKey::private_key_from_pem(private_key.as_bytes())?;
    let mut signer = Signer::new(MessageDigest::sha256(), &key)?;
    signer.set_rsa_padding(Padding::PKCS1)?;
    signer.update(signature_base.as_bytes())?;
    Ok(signer.sign_to_vec()?)
}
