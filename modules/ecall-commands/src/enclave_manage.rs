use crate::{prelude::*, EnclaveKeySelector, Error};
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::{Address, SealedEnclaveKey};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    InitEnclave(InitEnclaveInput),
    IASRemoteAttestation(IASRemoteAttestationInput),
    #[cfg(feature = "sgx-sw")]
    SimulateRemoteAttestation(SimulateRemoteAttestationInput),
}

impl EnclaveKeySelector for EnclaveManageCommand {
    fn get_enclave_key(&self) -> Option<Address> {
        match self {
            Self::InitEnclave(_) => None,
            Self::IASRemoteAttestation(input) => Some(input.target_enclave_key),
            #[cfg(feature = "sgx-sw")]
            Self::SimulateRemoteAttestation(input) => Some(input.target_enclave_key),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InitEnclaveInput;

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
pub enum EnclaveManageResult {
    InitEnclave(InitEnclaveResult),
    IASRemoteAttestation(IASRemoteAttestationResult),
    #[cfg(feature = "sgx-sw")]
    SimulateRemoteAttestation(SimulateRemoteAttestationResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEnclaveResult {
    pub pub_key: Vec<u8>,
    pub sealed_ek: SealedEnclaveKey,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct IASRemoteAttestationResult {
    pub report: EndorsedAttestationVerificationReport,
}

#[cfg(feature = "sgx-sw")]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SimulateRemoteAttestationResult {
    pub avr: attestation_report::AttestationVerificationReport,
}
