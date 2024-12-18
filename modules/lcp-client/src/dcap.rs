use dcap_rs::{types::VerifiedOutput, utils::hash::keccak256sum};

#[derive(Debug, Clone, PartialEq)]
pub struct DCAPVerifierCommit {
    pub output: VerifiedOutput,
    pub attestation_time: u64,
    pub sgx_intel_root_ca_hash: [u8; 32],
}

impl DCAPVerifierCommit {
    pub fn new(attestation_time: u64, output: VerifiedOutput, sgx_intel_root_ca: &[u8]) -> Self {
        Self {
            attestation_time,
            sgx_intel_root_ca_hash: keccak256sum(sgx_intel_root_ca),
            output,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = self.output.to_bytes();
        let mut aux: [u8; 40] = [0; 40];
        aux[..8].copy_from_slice(&self.attestation_time.to_be_bytes());
        aux[8..].copy_from_slice(&self.sgx_intel_root_ca_hash);
        output.extend_from_slice(aux.as_ref());
        output
    }

    pub fn from_bytes(slice: &[u8]) -> Self {
        let output = VerifiedOutput::from_bytes(&slice[..slice.len() - 48]);
        let mut attestation_time = [0; 8];
        attestation_time.copy_from_slice(&slice[slice.len() - 48..slice.len() - 40]);
        let mut sgx_intel_root_ca_hash = [0; 32];
        sgx_intel_root_ca_hash.copy_from_slice(&slice[slice.len() - 40..]);
        Self {
            output,
            attestation_time: u64::from_be_bytes(attestation_time),
            sgx_intel_root_ca_hash,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        keccak256sum(&self.to_bytes())
    }
}
