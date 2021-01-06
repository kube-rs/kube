#[cfg(feature = "native-tls")] use crate::Result;

#[cfg(feature = "native-tls")]
pub fn pkcs12_from_pem(pem: &[u8], password: &str) -> Result<Vec<u8>> {
    use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
    let x509 = X509::from_pem(&pem)?;
    let pkey = PKey::private_key_from_pem(&pem)?;
    let p12 = Pkcs12::builder().build(password, "kubeconfig", &pkey, &x509)?;
    let der = p12.to_der()?;
    Ok(der)
}
