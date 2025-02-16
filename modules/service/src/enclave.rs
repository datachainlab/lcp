use crate::service::AppService;
use attestation_report::RAType;
use crypto::Address;
use enclave_api::EnclaveProtoAPI;
use lcp_proto::lcp::service::enclave::v1::{
    query_server::Query, EnclaveKeyInfo, QueryAvailableEnclaveKeysRequest,
    QueryAvailableEnclaveKeysResponse, QueryEnclaveInfoRequest, QueryEnclaveInfoResponse,
    QueryEnclaveKeyRequest, QueryEnclaveKeyResponse,
};
use lcp_types::Mrenclave;
use store::transaction::CommitStore;
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl<E, S> Query for AppService<E, S>
where
    S: CommitStore + 'static,
    E: EnclaveProtoAPI<S> + 'static,
{
    async fn enclave_info(
        &self,
        _req: Request<QueryEnclaveInfoRequest>,
    ) -> Result<Response<QueryEnclaveInfoResponse>, Status> {
        self.enclave
            .metadata()
            .map(|metadata| {
                let res = QueryEnclaveInfoResponse {
                    mrenclave: metadata.enclave_css.body.enclave_hash.m.to_vec(),
                    enclave_debug: self.enclave.is_debug(),
                };
                Response::new(res)
            })
            .map_err(|e| Status::aborted(e.to_string()))
    }

    async fn available_enclave_keys(
        &self,
        req: Request<QueryAvailableEnclaveKeysRequest>,
    ) -> Result<Response<QueryAvailableEnclaveKeysResponse>, Status> {
        let mut res = QueryAvailableEnclaveKeysResponse::default();
        let req = req.into_inner();
        let keys = self
            .enclave
            .get_key_manager()
            .available_keys(
                Mrenclave::try_from(req.mrenclave).map_err(|e| Status::aborted(e.to_string()))?,
                req.enclave_debug,
                if req.ra_type == 0 {
                    None
                } else {
                    Some(
                        RAType::from_u32(req.ra_type)
                            .map_err(|e| Status::aborted(e.to_string()))?,
                    )
                },
            )
            .map_err(|e| Status::aborted(e.to_string()))?;
        for key in keys {
            res.keys
                .push(EnclaveKeyInfo::try_from(key).map_err(|e| Status::aborted(e.to_string()))?);
        }
        Ok(Response::new(res))
    }

    async fn enclave_key(
        &self,
        req: Request<QueryEnclaveKeyRequest>,
    ) -> Result<Response<QueryEnclaveKeyResponse>, Status> {
        let addr = Address::try_from(req.into_inner().enclave_key_address.as_slice())
            .map_err(|e| Status::aborted(e.to_string()))?;
        let key = self
            .enclave
            .get_key_manager()
            .load(addr)
            .map_err(|e| Status::aborted(e.to_string()))?;
        let key = EnclaveKeyInfo::try_from(key).map_err(|e| Status::aborted(e.to_string()))?;
        Ok(Response::new(QueryEnclaveKeyResponse { key: Some(key) }))
    }
}
