use crate::transmuter::BytesTransmuter;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use sgx_types::*;

#[derive(Serialize, Deserialize, Debug)]
pub enum RemoteAttestationCommand {
    InitQuote,
    GetIASSocket,
    GetQuote(GetQuoteInput),
    GetReportAttestationStatus(GetReportAttestationStatusInput),
}

#[derive(Serialize, Deserialize, Debug)]
pub enum RemoteAttestationResult {
    InitQuote(InitQuoteResult),
    GetIASSocket(GetIASSocketResult),
    GetQuote(GetQuoteResult),
    GetReportAttestationStatus(GetReportAttestationStatusResult),
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct InitQuoteResult {
    #[serde_as(as = "BytesTransmuter<sgx_target_info_t>")]
    pub target_info: sgx_target_info_t,
    pub epid_group_id: sgx_epid_group_id_t,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetIASSocketResult {
    pub fd: c_int,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct GetQuoteInput {
    pub sigrl: Vec<u8>,
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub report: sgx_report_t,
    #[serde_as(as = "BytesTransmuter<sgx_quote_sign_type_t>")]
    pub quote_type: sgx_quote_sign_type_t,
    #[serde_as(as = "BytesTransmuter<sgx_spid_t>")]
    pub spid: sgx_spid_t,
    #[serde_as(as = "BytesTransmuter<sgx_quote_nonce_t>")]
    pub nonce: sgx_quote_nonce_t,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct GetQuoteResult {
    #[serde_as(as = "BytesTransmuter<sgx_report_t>")]
    pub qe_report: sgx_report_t,
    pub quote: Vec<u8>,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct GetReportAttestationStatusInput {
    #[serde_as(as = "BytesTransmuter<sgx_platform_info_t>")]
    pub platform_blob: sgx_platform_info_t,
    pub enclave_trusted: i32,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct GetReportAttestationStatusResult {
    #[serde_as(as = "BytesTransmuter<sgx_status_t>")]
    pub ret: sgx_status_t,
    #[serde_as(as = "BytesTransmuter<sgx_update_info_bit_t>")]
    pub update_info: sgx_update_info_bit_t,
}
