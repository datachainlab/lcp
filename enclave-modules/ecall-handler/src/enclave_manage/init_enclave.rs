use crate::enclave_manage::Error;
use crate::prelude::*;
use crypto::KeyManager;
use ecall_commands::{CommandParams, InitEnclaveInput, InitEnclaveResult};

pub fn init_enclave(
    _: InitEnclaveInput,
    params: CommandParams,
) -> Result<InitEnclaveResult, Error> {
    let mut key_manager = KeyManager::new(params.home);
    let kp = match key_manager.get_enclave_key() {
        Some(kp) => kp,
        None => key_manager.create_enclave_key()?,
    };
    Ok(InitEnclaveResult {
        pub_key: kp.get_pubkey().as_bytes().to_vec(),
    })
}
