use attestation_report::DCAPQuote;
use dcap_pcs::client::PCSClient;
use dcap_quote_verifier::verifier::{QuoteVerificationOutput, Status};
use dcap_quote_verifier::{collateral::QvCollateral, types::quotes::CertData};
use lcp_types::proto::lcp::service::enclave::v1::{QvCollateral as ProtoQvCollateral, Validity};
use std::ops::Deref;

pub struct ValidatedPCSClient {
    client: PCSClient,
    expected_tcb_evaluation_data_number: Option<u32>,
}

impl ValidatedPCSClient {
    pub fn new(client: PCSClient, expected_tcb_evaluation_data_number: Option<u32>) -> Self {
        Self {
            client,
            expected_tcb_evaluation_data_number,
        }
    }

    pub fn validate_and_get_collateral(
        &self,
        is_sgx: bool,
        qe_cert_data: &CertData,
    ) -> Result<QvCollateral, anyhow::Error> {
        let collateral = self.client.get_collateral(is_sgx, qe_cert_data)?;
        if let Some(expected_tcb_evaluation_data_number) = self.expected_tcb_evaluation_data_number
        {
            let tcb_info_tcb_evaluation_data_number = collateral
                .get_tcb_info_v3()?
                .tcb_info
                .tcb_evaluation_data_number;
            if tcb_info_tcb_evaluation_data_number != expected_tcb_evaluation_data_number {
                return Err(anyhow::anyhow!(
                    "TCBInfo: the number of TCB evaluation data is not as expected: {} != {}",
                    tcb_info_tcb_evaluation_data_number,
                    expected_tcb_evaluation_data_number
                ));
            }
            let qe_identity_tcb_evaluation_data_number = collateral
                .get_qe_identity_v2()?
                .enclave_identity
                .tcb_evaluation_data_number;
            if qe_identity_tcb_evaluation_data_number != expected_tcb_evaluation_data_number {
                return Err(anyhow::anyhow!(
                    "QEIdentity: the number of TCB evaluation data is not as expected: {} != {}",
                    qe_identity_tcb_evaluation_data_number,
                    expected_tcb_evaluation_data_number
                ));
            }
        }
        Ok(collateral)
    }
}

impl Deref for ValidatedPCSClient {
    type Target = PCSClient;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

/// A list of TCB statuses and advisory IDs that are allowed.
#[derive(Debug, Clone, Default)]
pub struct QVResultAllowList {
    pub tcb_statuses: Vec<Status>,
    pub advisory_ids: Vec<String>,
}

impl QVResultAllowList {
    pub fn new(tcb_statuses: Vec<Status>, advisory_ids: Vec<String>) -> Self {
        Self {
            tcb_statuses,
            advisory_ids,
        }
    }

    pub fn is_allowed_tcb_status(&self, tcb_status: Status) -> Option<bool> {
        if self.tcb_statuses.is_empty() {
            None
        } else {
            Some(self.tcb_statuses.contains(&tcb_status))
        }
    }

    pub fn is_allowed_advisory_ids(&self, advisory_ids: &[String]) -> Option<bool> {
        if self.advisory_ids.is_empty() {
            None
        } else {
            Some(advisory_ids.iter().all(|id| self.advisory_ids.contains(id)))
        }
    }
}

#[derive(Debug)]
pub struct DCAPRemoteAttestationResult {
    pub raw_quote: Vec<u8>,
    pub output: QuoteVerificationOutput,
    pub collateral: QvCollateral,
}

impl DCAPRemoteAttestationResult {
    /// Converts the DCAPRemoteAttestationResult to a DCAPQuote
    pub fn to_ra_quote(&self) -> DCAPQuote {
        DCAPQuote {
            raw: self.raw_quote.clone(),
            fmspc: self.output.fmspc,
            status: self.output.status.to_string(),
            advisory_ids: self.output.advisory_ids.clone(),
            validity: Validity {
                not_before: self.output.validity.not_before,
                not_after: self.output.validity.not_after,
            },
            collateral: ProtoQvCollateral {
                tcb_info_json: self.collateral.tcb_info_json.clone(),
                qe_identity_json: self.collateral.qe_identity_json.clone(),
                sgx_intel_root_ca_der: self.collateral.sgx_intel_root_ca_der.clone(),
                sgx_tcb_signing_der: self.collateral.sgx_tcb_signing_der.clone(),
                sgx_intel_root_ca_crl_der: self.collateral.sgx_intel_root_ca_crl_der.clone(),
                sgx_pck_crl_der: self.collateral.sgx_pck_crl_der.clone(),
            },
        }
    }
}
