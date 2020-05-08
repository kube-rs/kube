use std::{
    env, fs,
    path::{Path, PathBuf},
};

use crate::{error::ConfigError, Error, Result};
use chrono::{DateTime, Utc};
use dirs::home_dir;

const KUBECONFIG: &str = "KUBECONFIG";

/// Search the kubeconfig file
pub fn find_kubeconfig() -> Result<PathBuf> {
    kubeconfig_path()
        .or_else(default_kube_path)
        .ok_or_else(|| ConfigError::NoKubeconfigPath.into())
}

/// Returns kubeconfig path from specified environment variable.
pub fn kubeconfig_path() -> Option<PathBuf> {
    env::var_os(KUBECONFIG).map(PathBuf::from)
}

/// Returns kubeconfig path from `$HOME/.kube/config`.
pub fn default_kube_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".kube").join("config"))
}

pub fn data_or_file_with_base64<P: AsRef<Path>>(data: &Option<String>, file: &Option<P>) -> Result<Vec<u8>> {
    match (data, file) {
        (Some(d), _) => base64::decode(&d)
            .map_err(ConfigError::Base64Decode)
            .map_err(Error::Kubeconfig),
        (_, Some(f)) => {
            let f = (*f).as_ref();
            let abs_file = if f.is_absolute() {
                f.to_path_buf()
            } else {
                find_kubeconfig().and_then(|cfg| {
                    cfg.parent()
                        .map(|kubedir| kubedir.join(f))
                        .ok_or_else(|| ConfigError::NoAbsolutePath { path: f.into() }.into())
                })?
            };
            // dbg!(&abs_file);
            fs::read(&abs_file).map_err(|source| {
                ConfigError::ReadFile {
                    path: abs_file,
                    source,
                }
                .into()
            })
        }
        _ => Err(ConfigError::NoBase64FileOrData.into()),
    }
}

pub fn data_or_file<P: AsRef<Path>>(data: &Option<String>, file: &Option<P>) -> Result<String> {
    match (data, file) {
        (Some(d), _) => Ok(d.to_string()),
        (_, Some(f)) => fs::read_to_string(f).map_err(|source| {
            ConfigError::ReadFile {
                path: f.as_ref().into(),
                source,
            }
            .into()
        }),
        _ => Err(ConfigError::NoFileOrData.into()),
    }
}

pub fn is_expired(timestamp: &str) -> bool {
    let ts = DateTime::parse_from_rfc3339(timestamp).unwrap();
    let now = DateTime::parse_from_rfc3339(&Utc::now().to_rfc3339()).unwrap();
    ts < now
}

#[cfg(test)]
mod tests {
    extern crate tempfile;
    use super::*;
    use crate::config::utils;
    use std::io::Write;

    #[test]
    fn test_kubeconfig_path() {
        let expect_str = "/fake/.kube/config";
        env::set_var(KUBECONFIG, expect_str);
        assert_eq!(PathBuf::from(expect_str), kubeconfig_path().unwrap());
    }

    #[test]
    fn test_data_or_file() {
        let data = "fake_data";
        let file = "fake_file";
        let mut tmpfile = tempfile::NamedTempFile::new().unwrap();
        write!(tmpfile, "{}", file).unwrap();

        let actual = utils::data_or_file(&Some(data.to_string()), &Some(tmpfile.path()));
        assert_eq!(actual.ok().unwrap(), data.to_string());

        let actual = utils::data_or_file(&None, &Some(tmpfile.path()));
        assert_eq!(actual.ok().unwrap(), file.to_string());

        let actual = utils::data_or_file(&None, &None::<String>);
        assert!(actual.is_err());
    }
}
