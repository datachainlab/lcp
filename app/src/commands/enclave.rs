use crate::{enclave::load_enclave, opts::Opts};
use anyhow::{bail, Result};
use clap::Parser;
use enclave_api::EnclavePrimitiveAPI;
use log::*;
use settings::{AVR_KEY_PATH, SEALED_ENCLAVE_KEY_PATH};
use std::{
    fs::{remove_file, OpenOptions},
    io::Write,
    path::PathBuf,
};

// `enclave` subcommand
#[derive(Debug, Parser)]
pub enum EnclaveCmd {
    #[clap(about = "Initialize an enclave key")]
    InitKey(InitKey),
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct InitKey {
    /// Path to the enclave binary
    #[clap(long = "enclave", help = "Path to enclave binary")]
    pub enclave: Option<PathBuf>,

    /// Boolean flag to overwrite an enclave key and AVR if it already exists
    #[clap(
        long = "force",
        default_value = "false",
        help = "Boolean flag to overwrite an enclave key and AVR if it already exists."
    )]
    pub force: bool,
}

impl EnclaveCmd {
    pub fn run(&self, opts: &Opts) -> Result<()> {
        match self {
            EnclaveCmd::InitKey(cmd) => {
                let spid = std::env::var("SPID")?;
                let ias_key = std::env::var("IAS_KEY")?;

                let home = opts.get_home();
                if !home.exists() {
                    std::fs::create_dir_all(&home)?;
                    info!("created home directory: {:?}", home);
                }

                let ek_path = home.join(SEALED_ENCLAVE_KEY_PATH);
                if ek_path.exists() {
                    if cmd.force {
                        remove_file(&ek_path)?;
                    } else {
                        bail!(
                            "Init Key Failed: path of ek {:?} already exists",
                            ek_path.as_path(),
                        );
                    }
                }

                let avr_path = home.join(AVR_KEY_PATH);
                if avr_path.exists() {
                    if cmd.force {
                        remove_file(&avr_path)?;
                    } else {
                        bail!(
                            "Init Key Failed: path of avr {:?} already exists",
                            avr_path.as_path(),
                        );
                    }
                }

                let enclave = load_enclave(opts, cmd.enclave.as_ref())?;
                match enclave.init_enclave_key(spid.as_bytes(), ias_key.as_bytes()) {
                    Ok(res) => {
                        let s = serde_json::to_string(&res.report)?;
                        info!("successfully got the AVR");
                        // NOTE: Currently, enclave key and AVR file operations are not atomic.
                        // Therefore, if the service is running in the background, the service may read incomplete data (and its attempt will be failed).
                        // To solve this problem, consider using the traditional `rename` approach or a File DB such as rocksdb.
                        let mut f = OpenOptions::new()
                            .write(true)
                            .create_new(true)
                            .open(&avr_path)?;
                        f.write_all(s.as_bytes())?;
                        f.flush()?;
                        Ok(())
                    }
                    Err(e) => bail!("ECALL Enclave Failed {:?}!", e),
                }
            }
        }
    }
}
