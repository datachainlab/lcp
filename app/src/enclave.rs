use crate::opts::Opts;
use anyhow::{bail, Result};
use enclave_api::{Enclave, EnclaveProtoAPI};
use keymanager::EnclaveKeyManager;
use std::path::PathBuf;
use store::transaction::CommitStore;

pub trait EnclaveLoader<S: CommitStore> {
    fn load(&self, opts: &Opts, path: Option<&PathBuf>, debug: bool) -> Result<Enclave<S>>;
}

#[derive(Debug)]
pub struct DefaultEnclaveLoader<S: CommitStore>(std::marker::PhantomData<S>);

impl<S: CommitStore> EnclaveLoader<S> for DefaultEnclaveLoader<S>
where
    Enclave<S>: EnclaveProtoAPI<S>,
{
    fn load(&self, opts: &Opts, path: Option<&PathBuf>, debug: bool) -> Result<Enclave<S>> {
        let path = if let Some(path) = path {
            path.clone()
        } else {
            opts.default_enclave()
        };
        let env = host::get_environment().unwrap();
        let km = EnclaveKeyManager::new(&env.home)?;
        match Enclave::create(&path, debug, km, env.store.clone()) {
            Ok(enclave) => Ok(enclave),
            Err(x) => {
                bail!(
                    "Init Enclave Failed: status={} path={:?}",
                    x.as_str(),
                    path.as_path()
                );
            }
        }
    }
}

pub const fn build_enclave_loader<S: CommitStore>() -> DefaultEnclaveLoader<S>
where
    Enclave<S>: EnclaveProtoAPI<S>,
{
    DefaultEnclaveLoader(std::marker::PhantomData)
}
