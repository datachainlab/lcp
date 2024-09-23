use crate::prelude::*;
use crypto::{Address, EnclavePublicKey, SealedEnclaveKey};
use lcp_types::BytesTransmuter;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sgx_types::{sgx_report_t, sgx_target_info_t};

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    GenerateEnclaveKey(GenerateEnclaveKeyInput),
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GenerateEnclaveKeyInput {
    #[serde_as(as = "BytesTransmuter<sgx_target_info_t>")]
    pub target_info: sgx_target_info_t,
    pub operator: Option<Address>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageResponse {
    GenerateEnclaveKey(GenerateEnclaveKeyResponse),
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateEnclaveKeyResponse {
    pub pub_key: EnclavePublicKey,
    pub sealed_ek: SealedEnclaveKey,
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub report: sgx_report_t,
}
