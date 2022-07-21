#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use attestation_report::EndorsedAttestationReport;
use serde::{Deserialize, Serialize};
use std::vec::Vec;

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    InitEnclave(InitEnclaveInput),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEnclaveInput {
    pub spid: Vec<u8>,
    pub ias_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageResult {
    InitEnclave(InitEnclaveResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEnclaveResult {
    pub report: EndorsedAttestationReport,
}
