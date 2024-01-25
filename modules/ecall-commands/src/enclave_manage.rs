use crate::{prelude::*, EnclaveKeySelector, InputValidationError as Error};
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::{Address, EnclavePublicKey, SealedEnclaveKey};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    GenerateEnclaveKey(GenerateEnclaveKeyInput),
    IASRemoteAttestation(IASRemoteAttestationInput),
    #[cfg(feature = "sgx-sw")]
    SimulateRemoteAttestation(SimulateRemoteAttestationInput),
}

impl EnclaveKeySelector for EnclaveManageCommand {
    fn get_enclave_key(&self) -> Option<Address> {
        match self {
            Self::GenerateEnclaveKey(_) => None,
            Self::IASRemoteAttestation(input) => Some(input.target_enclave_key),
            #[cfg(feature = "sgx-sw")]
            Self::SimulateRemoteAttestation(input) => Some(input.target_enclave_key),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GenerateEnclaveKeyInput;

#[derive(Serialize, Deserialize, Debug)]
pub struct IASRemoteAttestationInput {
    pub target_enclave_key: Address,
    pub spid: Vec<u8>,
    pub ias_key: Vec<u8>,
}

impl IASRemoteAttestationInput {
    pub fn validate(&self) -> Result<(), Error> {
        if self.spid.len() == 32 && self.ias_key.len() == 32 {
            Ok(())
        } else {
            Err(Error::invalid_argument(
                "either or both of SPID and IAS_KEY are invalid".to_string(),
            ))
        }
    }
}

#[cfg(feature = "sgx-sw")]
#[derive(Serialize, Deserialize, Debug)]
pub struct SimulateRemoteAttestationInput {
    pub target_enclave_key: Address,
    pub advisory_ids: Vec<String>,
    pub isv_enclave_quote_status: String,
}

#[cfg(feature = "sgx-sw")]
impl SimulateRemoteAttestationInput {
    pub fn validate(&self) -> Result<(), Error> {
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageResponse {
    GenerateEnclaveKey(GenerateEnclaveKeyResponse),
    IASRemoteAttestation(IASRemoteAttestationResponse),
    #[cfg(feature = "sgx-sw")]
    SimulateRemoteAttestation(SimulateRemoteAttestationResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateEnclaveKeyResponse {
    pub pub_key: EnclavePublicKey,
    pub sealed_ek: SealedEnclaveKey,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct IASRemoteAttestationResponse {
    pub report: EndorsedAttestationVerificationReport,
}

#[cfg(feature = "sgx-sw")]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SimulateRemoteAttestationResponse {
    pub avr: attestation_report::AttestationVerificationReport,
}
