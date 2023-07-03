use crate::enclave_manage::Error;
use crate::prelude::*;
use crypto::{EnclaveKey, SealingKey};
use ecall_commands::{GenerateEnclaveKeyInput, GenerateEnclaveKeyResult};

pub(crate) fn generate_enclave_key(
    _: GenerateEnclaveKeyInput,
) -> Result<GenerateEnclaveKeyResult, Error> {
    let ek = EnclaveKey::new()?;
    let sealed_ek = ek.seal()?;
    Ok(GenerateEnclaveKeyResult {
        pub_key: ek.get_pubkey(),
        sealed_ek,
    })
}
