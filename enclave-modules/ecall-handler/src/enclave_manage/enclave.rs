use crate::enclave_manage::Error;
use crate::prelude::*;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{GenerateEnclaveKeyInput, GenerateEnclaveKeyResponse};

pub(crate) fn generate_enclave_key(
    _: GenerateEnclaveKeyInput,
) -> Result<GenerateEnclaveKeyResponse, Error> {
    let ek = EnclaveKey::new()?;
    let sealed_ek = ek.seal()?;
    Ok(GenerateEnclaveKeyResponse {
        pub_key: ek.get_pubkey(),
        sealed_ek,
    })
}
