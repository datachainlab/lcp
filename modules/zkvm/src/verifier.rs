use crate::create_groth16_receipt;
use crate::Error;
use risc0_zkvm::sha::Digest;

pub fn verify_groth16_proof(
    seal: Vec<u8>,
    image_id: impl Into<Digest>,
    journal: Vec<u8>,
) -> Result<(), Error> {
    let expected_selector = &seal[..4];
    let data = &seal[4..];
    let receipt = create_groth16_receipt(data.to_vec(), image_id, journal);
    let selector = receipt.verifier_parameters.as_bytes()[..4].to_vec();
    if expected_selector != selector {
        return Err(Error::unexpected_selector(
            expected_selector.to_vec(),
            selector,
        ));
    }
    receipt
        .verify_integrity()
        .map_err(|e| Error::groth16_verification_error(e.to_string()))
}
