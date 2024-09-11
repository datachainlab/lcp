use crate::transmuter::BytesTransmuter;
use crate::{prelude::*, EnclaveKeySelector};
use crypto::{Address, EnclavePublicKey, SealedEnclaveKey};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sgx_types::{sgx_report_t, sgx_target_info_t};

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    GenerateEnclaveKey(GenerateEnclaveKeyInput),
    CreateReport(CreateReportInput),
}

impl EnclaveKeySelector for EnclaveManageCommand {
    fn get_enclave_key(&self) -> Option<Address> {
        match self {
            Self::GenerateEnclaveKey(_) => None,
            Self::CreateReport(input) => Some(input.target_enclave_key),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GenerateEnclaveKeyInput;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct CreateReportInput {
    #[serde_as(as = "BytesTransmuter<sgx_target_info_t>")]
    pub target_info: sgx_target_info_t,
    pub target_enclave_key: Address,
    pub operator: Option<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageResponse {
    GenerateEnclaveKey(GenerateEnclaveKeyResponse),
    CreateReport(CreateReportResponse),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateEnclaveKeyResponse {
    pub pub_key: EnclavePublicKey,
    pub sealed_ek: SealedEnclaveKey,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateReportResponse {
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub report: sgx_report_t,
}
