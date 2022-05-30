use crate::context::Context;
use crate::enclave_manage;
use crate::light_client;
use crate::{HandlerError as Error, Result};
use anyhow::anyhow;
use enclave_commands::{Command, CommandResult};
use enclave_crypto::EnclaveKey;
use enclave_light_client::LightClientSource;
use log::*;
use std::format;
use store::Store;

pub fn dispatch<'l, S: Store, L: LightClientSource<'l>>(
    ek: Option<&EnclaveKey>,
    mut store: S,
    command: Command,
) -> Result<CommandResult> {
    let res = match command {
        Command::EnclaveManage(cmd) => enclave_manage::dispatch(cmd)?,
        _ => {
            let mut ctx = match ek {
                None => return Err(Error::OtherError(anyhow!("ek must not be nil"))),
                Some(ek) => {
                    store
                        .load_and_verify(&ek.get_pubkey())
                        .map_err(Error::StoreError)?;
                    Context::new(&mut store, &ek)
                }
            };
            match command {
                Command::LightClient(cmd) => match light_client::dispatch::<_, L>(&mut ctx, cmd) {
                    Ok(res) => {
                        let commit = store
                            .commit_and_sign(ek.unwrap())
                            .map_err(Error::StoreError)?;
                        info!("commit={:?}", commit);
                        res
                    }
                    Err(e) => {
                        store.rollback();
                        return Err(Error::OtherError(anyhow!(
                            "failed to execute the command: {}",
                            e
                        )));
                    }
                },
                _ => unreachable!(),
            }
        }
    };
    Ok(res)
}
