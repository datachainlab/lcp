mod errors;
#[cfg(feature = "prover")]
pub mod prover;
#[cfg(feature = "verifier")]
pub mod verifier;
pub use crate::errors::Error;
pub use risc0_zkvm;
use risc0_zkvm::{
    sha::{Digest, Digestible},
    Groth16Receipt, Groth16ReceiptVerifierParameters, InnerReceipt, MaybePruned, ReceiptClaim,
};

/// Encode the seal of the given receipt for use with EVM smart contract verifiers.
///
/// Appends the verifier selector, determined from the first 4 bytes of the verifier parameters
/// including the Groth16 verification key and the control IDs that commit to the RISC Zero
/// circuits.
pub fn encode_seal_selector(receipt: &risc0_zkvm::Receipt) -> Result<([u8; 4], Vec<u8>), Error> {
    match receipt.inner.clone() {
        InnerReceipt::Fake(receipt) => {
            let seal = receipt.claim.digest().as_bytes().to_vec();
            Ok((Default::default(), seal))
        }
        InnerReceipt::Groth16(receipt) => {
            let mut selector = [0u8; 4];
            selector.copy_from_slice(&receipt.verifier_parameters.as_bytes()[..4]);
            Ok((selector, receipt.seal))
        }
        _ => Err(Error::unsupported_receipt_type(format!(
            "{:?}",
            receipt.inner
        ))),
    }
}

pub fn create_groth16_receipt(
    seal: Vec<u8>,
    image_id: impl Into<Digest>,
    journal: Vec<u8>,
) -> Groth16Receipt<ReceiptClaim> {
    let claim = MaybePruned::Value(ReceiptClaim::ok(image_id, journal));
    Groth16Receipt::new(
        seal,
        claim,
        Groth16ReceiptVerifierParameters::default().digest(),
    )
}
