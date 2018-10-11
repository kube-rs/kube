use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use base64;
use dirs::home_dir;
use failure::Error;

const KUBECONFIG: &str = "KUBECONFIG";

pub fn kube_path() -> Option<PathBuf> {
    env::var_os(KUBECONFIG).map(PathBuf::from)
}

pub fn default_kube_path() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".kube").join("config"))
}

pub fn data_or_file(
    data: &Option<String>,
    file: &Option<String>,
) -> Result<Option<Vec<u8>>, Error> {
    match (data, file) {
        (Some(d), _) => Ok(Some(base64::decode(&d)?)),
        (_, Some(f)) => {
            let mut b = vec![];
            let mut ff = File::open(f)?;
            ff.read_to_end(&mut b)?;
            Ok(Some(b))
        }
        _ => Ok(None),
    }
}
