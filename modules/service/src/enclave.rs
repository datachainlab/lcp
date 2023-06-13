use crate::service::AppService;
use attestation_report::EndorsedAttestationVerificationReport;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::enclave::v1::{
    query_server::Query, QueryAttestedVerificationReportRequest,
    QueryAttestedVerificationReportResponse,
};
use settings::AVR_KEY_PATH;
use std::fs::File;
use std::io::Read;
use store::transaction::CommitStore;
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl<E, S> Query for AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    async fn attested_verification_report(
        &self,
        _: Request<QueryAttestedVerificationReportRequest>,
    ) -> Result<Response<QueryAttestedVerificationReportResponse>, Status> {
        let path = self.home.join(AVR_KEY_PATH);
        let mut json = String::new();
        File::open(path)?.read_to_string(&mut json)?;

        let ereport: EndorsedAttestationVerificationReport =
            serde_json::from_str(&json).map_err(|e| Status::internal(e.to_string()))?;

        let quote = ereport
            .get_avr()
            .map_err(|e| Status::internal(e.to_string()))?
            .parse_quote()
            .map_err(|e| Status::internal(e.to_string()))?;
        let metadata = self
            .enclave
            .metadata()
            .map_err(|e| Status::internal(e.to_string()))?;
        quote
            .match_metadata(&metadata)
            .map_err(|e| Status::internal(e.to_string()))?;
        let address = quote
            .get_enclave_key_address()
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(QueryAttestedVerificationReportResponse {
            enclave_address: address.into(),
            report: ereport.avr,
            signature: ereport.signature,
            signing_cert: ereport.signing_cert,
        }))
    }
}
