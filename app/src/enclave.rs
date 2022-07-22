use crate::opts::Opts;
use anyhow::{bail, Result};
use enclave_api::Enclave;
use std::path::PathBuf;

pub(crate) fn load_enclave(opts: &Opts, path: Option<&PathBuf>) -> Result<Enclave> {
    let path = if let Some(path) = path {
        path.clone()
    } else {
        opts.default_enclave()
    };
    match host::enclave::load_enclave(&path) {
        Ok(enclave) => Ok(Enclave::new(
            enclave,
            opts.get_home().to_str().unwrap().to_string(),
        )),
        Err(x) => {
            bail!(
                "Init Enclave Failed: status={} path={:?}",
                x.as_str(),
                path.as_path()
            );
        }
    }
}
