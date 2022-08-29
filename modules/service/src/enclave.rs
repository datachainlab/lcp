use std::fs::File;
use std::io::Read;

use crate::service::AppService;
use attestation_report::{parse_quote_from_report, EndorsedAttestationReport};
use lcp_proto::lcp::service::enclave::v1::{
    query_server::Query, QueryAttestedVerificationReportRequest,
    QueryAttestedVerificationReportResponse,
};

use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Query for AppService {
    async fn attested_verification_report(
        &self,
        _: Request<QueryAttestedVerificationReportRequest>,
    ) -> Result<Response<QueryAttestedVerificationReportResponse>, Status> {
        let path = self.home.join(settings::AVR_KEY_PATH);
        let mut json = String::new();
        File::open(path)?.read_to_string(&mut json)?;

        let avr: EndorsedAttestationReport =
            serde_json::from_str(&json).map_err(|e| Status::internal(e.to_string()))?;

        let quote =
            parse_quote_from_report(&avr.report).map_err(|e| Status::internal(e.to_string()))?;
        let address = quote
            .get_enclave_key_address()
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(QueryAttestedVerificationReportResponse {
            enclave_address: address.into(),
            report: avr.report,
            signature: avr.signature,
            signing_cert: avr.signing_cert,
        }))
    }
}
