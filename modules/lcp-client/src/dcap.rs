use dcap_rs::{types::VerifiedOutput, utils::hash::keccak256sum};

#[derive(Debug, Clone, PartialEq)]
pub struct DCAPVerifierCommit {
    pub attestation_time: u64,
    pub sgx_intel_root_ca_hash: [u8; 32],
    pub output: VerifiedOutput,
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
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.attestation_time.to_le_bytes());
        bytes.extend_from_slice(&self.sgx_intel_root_ca_hash);
        bytes.append(&mut output);
        bytes
    }

    pub fn from_bytes(slice: &[u8]) -> Self {
        let mut attestation_time = [0; 8];
        attestation_time.copy_from_slice(&slice[0..8]);
        let mut sgx_intel_root_ca_hash = [0; 32];
        sgx_intel_root_ca_hash.copy_from_slice(&slice[8..40]);
        let output = VerifiedOutput::from_bytes(&slice[40..]);
        Self {
            attestation_time: u64::from_le_bytes(attestation_time),
            sgx_intel_root_ca_hash,
            output,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        keccak256sum(&self.to_bytes())
    }
}
