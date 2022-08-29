use std::fs::File;
use std::io::Read;

use crate::service::AppService;
use attestation_report::EndorsedAttestationReport;
use lcp_proto::lcp::service::enclave::v1::{
    query_server::Query, QueryAttestedEnclaveKeyRequest, QueryAttestedEnclaveKeyResponse,
};

use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Query for AppService {
    async fn attested_enclave_key(
        &self,
        _: Request<QueryAttestedEnclaveKeyRequest>,
    ) -> Result<Response<QueryAttestedEnclaveKeyResponse>, Status> {
        let path = self.home.join(settings::AVR_KEY_PATH);
        let mut json = String::new();
        File::open(path)?.read_to_string(&mut json)?;

        let avr: EndorsedAttestationReport =
            serde_json::from_str(&json).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(QueryAttestedEnclaveKeyResponse {
            enclave_public_key: todo!(),
            report: avr.report,
            signature: avr.signature,
            signing_cert: avr.signing_cert,
        }))
    }
}
