use crate::service::AppService;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::enclave::v1::{
    query_server::Query, QueryAttestedEnclaveKeyRequest, QueryAttestedEnclaveKeyResponse,
};

use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl Query for AppService {
    async fn attested_enclave_key(
        &self,
        request: Request<QueryAttestedEnclaveKeyRequest>,
    ) -> Result<Response<QueryAttestedEnclaveKeyResponse>, Status> {
        match self
            .enclave
            .proto_query_attested_enclave_key(request.into_inner())
        {
            Ok(res) => Ok(Response::new(res)),
            Err(e) => Err(Status::aborted(e.to_string())),
        }
    }
}
