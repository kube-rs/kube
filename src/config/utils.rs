use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use base64;
use chrono::{DateTime, Utc};
use dirs::home_dir;
use failure::Error;

const KUBECONFIG: &str = "KUBECONFIG";

/// Search the kubeconfig file
pub fn find_kubeconfig() -> Result<PathBuf, Error> {
    kubeconfig_path()
        .or_else(default_kube_path)
        .ok_or_else(|| format_err!("Failed to find path of kubeconfig"))
}

/// Returns kubeconfig path from specified environment variable.
pub fn kubeconfig_path() -> Option<PathBuf> {
    env::var_os(KUBECONFIG).map(PathBuf::from)
}

/// Returns kubeconfig path from `$HOME/.kube/config`.
pub fn default_kube_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".kube").join("config"))
}

pub fn data_or_file_with_base64<P: AsRef<Path>>(
    data: &Option<String>,
    file: &Option<P>,
) -> Result<Vec<u8>, Error> {
    match (data, file) {
        (Some(d), _) => base64::decode(&d).map_err(Error::from),
        (_, Some(f)) => {
            let f = (*f).as_ref();
            let abs_file = if f.is_absolute() {
                f.to_path_buf()
            } else {
                find_kubeconfig()
                .and_then(|cfg|
                    cfg.parent()
                    .map(|kubedir| kubedir.join(f))
                    .ok_or_else(|| format_err!("Failed to compute the absolute path of '{:?}'", f))
                )?
            };
            // dbg!(&abs_file);
            let mut ff = File::open(&abs_file)?;
            let mut b = vec![];
            ff.read_to_end(&mut b)?;
            Ok(b)
        }
        _ => Err(format_err!("Failed to get data/file with base64 format")),
    }
}

pub fn data_or_file<P: AsRef<Path>>(
    data: &Option<String>,
    file: &Option<P>,
) -> Result<String, Error> {
    match (data, file) {
        (Some(d), _) => Ok(d.to_string()),
        (_, Some(f)) => {
            let mut s = String::new();
            let mut ff = File::open(f)?;
            ff.read_to_string(&mut s)?;
            Ok(s)
        }
        _ => Err(format_err!("Failed to get data/file")),
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
