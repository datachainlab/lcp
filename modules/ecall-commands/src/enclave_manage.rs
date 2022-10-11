use crate::prelude::*;
use attestation_report::EndorsedAttestationVerificationReport;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageCommand {
    InitEnclave(InitEnclaveInput),
    IASRemoteAttestation(IASRemoteAttestationInput),
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct InitEnclaveInput;

#[derive(Serialize, Deserialize, Debug)]
pub struct IASRemoteAttestationInput {
    pub spid: Vec<u8>,
    pub ias_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum EnclaveManageResult {
    InitEnclave(InitEnclaveResult),
    IASRemoteAttestation(IASRemoteAttestationResult),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct InitEnclaveResult {
    pub pub_key: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct IASRemoteAttestationResult {
    pub report: EndorsedAttestationVerificationReport,
}
