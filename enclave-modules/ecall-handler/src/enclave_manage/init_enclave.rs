use crate::enclave_manage::Error;
use crate::prelude::*;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{CommandParams, InitEnclaveInput, InitEnclaveResult};

pub(crate) fn init_enclave(
    _: InitEnclaveInput,
    params: CommandParams,
) -> Result<InitEnclaveResult, Error> {
    assert!(params.sealed_ek.len() == 0);
    let ek = EnclaveKey::new()?;
    let sealed_ek = ek.seal()?;
    Ok(InitEnclaveResult {
        pub_key: ek.get_pubkey().as_bytes().to_vec(),
        sealed_ek,
    })
}
