use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{error::ConfigError, Error, Result};
use dirs::home_dir;

/// Returns kubeconfig path from `$HOME/.kube/config`.
pub fn default_kube_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".kube").join("config"))
}

pub fn data_or_file_with_base64<P: AsRef<Path>>(data: &Option<String>, file: &Option<P>) -> Result<Vec<u8>> {
    let mut blob = match (data, file) {
        (Some(d), _) => base64::decode(&d)
            .map_err(ConfigError::Base64Decode)
            .map_err(Error::Kubeconfig),
        (_, Some(f)) => read_file(f),
        _ => Err(ConfigError::NoBase64FileOrData.into()),
    };
    //Ensure there is a trailing newline in the blob
    //Don't bother if the blob is empty
    if let Ok(buf) = &mut blob {
        if buf.last().map(|end| *end != b'\n').unwrap_or(false) {
            buf.push(b'\n');
        }
    }
    blob
}

pub fn read_file<P: AsRef<Path>>(file: P) -> Result<Vec<u8>> {
    fs::read(&file).map_err(|source| {
        ConfigError::ReadFile {
            path: file.as_ref().into(),
            source,
        }
        .into()
    })
}

pub fn read_file_to_string<P: AsRef<Path>>(file: P) -> Result<String> {
    fs::read_to_string(&file).map_err(|source| {
        ConfigError::ReadFile {
            path: file.as_ref().into(),
            source,
        }
        .into()
    })
}

pub fn certs(data: &[u8]) -> Vec<Vec<u8>> {
    pem::parse_many(data)
        .into_iter()
        .filter_map(|p| {
            if p.tag == "CERTIFICATE" {
                Some(p.contents)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}
