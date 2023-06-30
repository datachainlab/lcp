use crate::enclave_manage::Error;
use crate::prelude::*;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{CommandContext, InitEnclaveInput, InitEnclaveResult};

pub(crate) fn init_enclave(
    cctx: CommandContext,
    _: InitEnclaveInput,
) -> Result<InitEnclaveResult, Error> {
    assert!(cctx.sealed_ek.len() == 0);
    let ek = EnclaveKey::new()?;
    let sealed_ek = ek.seal()?;
    Ok(InitEnclaveResult {
        pub_key: ek.get_pubkey().as_bytes().to_vec(),
        sealed_ek,
    })
}
