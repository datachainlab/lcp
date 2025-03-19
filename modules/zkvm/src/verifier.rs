use crate::create_groth16_receipt;
use crate::Error;
use risc0_zkvm::sha::Digest;

pub fn verify_groth16_proof(
    selector: [u8; 4],
    seal: Vec<u8>,
    image_id: impl Into<Digest>,
    journal: Vec<u8>,
) -> Result<(), Error> {
    let receipt = create_groth16_receipt(seal, image_id, journal);
    let expected_selector = &receipt.verifier_parameters.as_bytes()[..4];
    if selector != expected_selector {
        return Err(Error::unexpected_selector(
            expected_selector.to_vec(),
            selector.to_vec(),
        ));
    }
    receipt
        .verify_integrity()
        .map_err(|e| Error::groth16_verification_error(e.to_string()))
}
