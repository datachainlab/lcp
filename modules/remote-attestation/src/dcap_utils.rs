use attestation_report::DCAPQuote;
use dcap_quote_verifier::collateral::QvCollateral;
use dcap_quote_verifier::verifier::QuoteVerificationOutput;
use lcp_types::proto::lcp::service::enclave::v1::{QvCollateral as ProtoQvCollateral, Validity};

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
