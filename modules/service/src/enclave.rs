use crate::service::AppService;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::enclave::v1::{
    query_server::Query, QueryAttestedVerificationReportRequest,
    QueryAttestedVerificationReportResponse,
};
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
        todo!()
    }
}
