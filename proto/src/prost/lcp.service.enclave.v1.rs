/// Request for getting the enclave information.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryEnclaveInfoRequest {}
/// Response for getting the enclave information.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryEnclaveInfoResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub mrenclave: ::prost::alloc::vec::Vec<u8>,
    #[prost(bool, tag = "2")]
    pub enclave_debug: bool,
}
/// Request for getting the attested enclave keys corresponding to the specified MRENCLAVE.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryAvailableEnclaveKeysRequest {
    /// MRENCLAVE of the enclave that generates the EK.
    #[prost(bytes = "vec", tag = "1")]
    pub mrenclave: ::prost::alloc::vec::Vec<u8>,
    /// Debug flag of the enclave that generates the EK.
    #[prost(bool, tag = "2")]
    pub enclave_debug: bool,
    /// Remote attestation type.
    ///
    /// | Type            | Value |
    /// |-----------------|-------|
    /// | IAS             |   1   |
    /// | DCAP            |   2   |
    /// | ZKDCAPRisc0     |   3   |
    /// | MockZKDCAPRisc0 |   4   |
    #[prost(uint32, tag = "3")]
    pub ra_type: u32,
}
/// Response for getting the attested enclave keys.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryAvailableEnclaveKeysResponse {
    #[prost(message, repeated, tag = "1")]
    pub keys: ::prost::alloc::vec::Vec<EnclaveKeyInfo>,
}
/// Enclave key information contains the RA type specific information.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EnclaveKeyInfo {
    #[prost(oneof = "enclave_key_info::KeyInfo", tags = "1, 2, 3")]
    pub key_info: ::core::option::Option<enclave_key_info::KeyInfo>,
}
/// Nested message and enum types in `EnclaveKeyInfo`.
pub mod enclave_key_info {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum KeyInfo {
        #[prost(message, tag = "1")]
        Ias(super::IasEnclaveKeyInfo),
        #[prost(message, tag = "2")]
        Dcap(super::DcapEnclaveKeyInfo),
        #[prost(message, tag = "3")]
        Zkdcap(super::ZkdcapEnclaveKeyInfo),
    }
}
/// Enclave key information with IAS report.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct IasEnclaveKeyInfo {
    #[prost(bytes = "vec", tag = "1")]
    pub enclave_key_address: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "2")]
    pub report: ::prost::alloc::string::String,
    #[prost(uint64, tag = "3")]
    pub attestation_time: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub signing_cert: ::prost::alloc::vec::Vec<u8>,
}
/// Enclave key information with DCAP quote and supplemental data.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DcapEnclaveKeyInfo {
    #[prost(bytes = "vec", tag = "1")]
    pub enclave_key_address: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub quote: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub fmspc: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "4")]
    pub validity: ::core::option::Option<Validity>,
    #[prost(string, tag = "5")]
    pub tcb_status: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "6")]
    pub advisory_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "7")]
    pub collateral: ::core::option::Option<QvCollateral>,
}
/// Validity Period
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Validity {
    #[prost(uint64, tag = "1")]
    pub not_before: u64,
    #[prost(uint64, tag = "2")]
    pub not_after: u64,
}
/// Enclave key information with zkDCAP proof and DCAP attestation info.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ZkdcapEnclaveKeyInfo {
    #[prost(message, optional, tag = "1")]
    pub dcap: ::core::option::Option<DcapEnclaveKeyInfo>,
    #[prost(message, optional, tag = "2")]
    pub zkp: ::core::option::Option<ZkvmProof>,
}
/// ZKVM proof
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ZkvmProof {
    #[prost(oneof = "zkvm_proof::Proof", tags = "1")]
    pub proof: ::core::option::Option<zkvm_proof::Proof>,
}
/// Nested message and enum types in `ZKVMProof`.
pub mod zkvm_proof {
    #[derive(::serde::Serialize, ::serde::Deserialize)]
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Proof {
        #[prost(message, tag = "1")]
        Risc0(super::Risc0ZkvmProof),
    }
}
/// RISC Zero zkVM proof for zkDCAP
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Risc0ZkvmProof {
    #[prost(bytes = "vec", tag = "1")]
    pub image_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub selector: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub seal: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub output: ::prost::alloc::vec::Vec<u8>,
}
/// Collateral information for the DCAP quote verification.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QvCollateral {
    #[prost(string, tag = "1")]
    pub tcb_info_json: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub qe_identity_json: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "3")]
    pub sgx_intel_root_ca_der: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub sgx_tcb_signing_der: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub sgx_intel_root_ca_crl_der: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "6")]
    pub sgx_pck_crl_der: ::prost::alloc::vec::Vec<u8>,
}
/// Request for getting the enclave key information.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryEnclaveKeyRequest {
    #[prost(bytes = "vec", tag = "1")]
    pub enclave_key_address: ::prost::alloc::vec::Vec<u8>,
}
/// Response for getting the enclave key information.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryEnclaveKeyResponse {
    #[prost(message, optional, tag = "1")]
    pub key: ::core::option::Option<EnclaveKeyInfo>,
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod query_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct QueryClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl QueryClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> QueryClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> QueryClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            QueryClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Get the enclave information loaded in the service.
        pub async fn enclave_info(
            &mut self,
            request: impl tonic::IntoRequest<super::QueryEnclaveInfoRequest>,
        ) -> Result<tonic::Response<super::QueryEnclaveInfoResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/lcp.service.enclave.v1.Query/EnclaveInfo",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Get the available enclave keys for matching the
        /// specified MRENCLAVE and debug flag and RA type.
        pub async fn available_enclave_keys(
            &mut self,
            request: impl tonic::IntoRequest<super::QueryAvailableEnclaveKeysRequest>,
        ) -> Result<
            tonic::Response<super::QueryAvailableEnclaveKeysResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/lcp.service.enclave.v1.Query/AvailableEnclaveKeys",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Get the enclave key information for the specified enclave key address.
        pub async fn enclave_key(
            &mut self,
            request: impl tonic::IntoRequest<super::QueryEnclaveKeyRequest>,
        ) -> Result<tonic::Response<super::QueryEnclaveKeyResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/lcp.service.enclave.v1.Query/EnclaveKey",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod query_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with QueryServer.
    #[async_trait]
    pub trait Query: Send + Sync + 'static {
        /// Get the enclave information loaded in the service.
        async fn enclave_info(
            &self,
            request: tonic::Request<super::QueryEnclaveInfoRequest>,
        ) -> Result<tonic::Response<super::QueryEnclaveInfoResponse>, tonic::Status>;
        /// Get the available enclave keys for matching the
        /// specified MRENCLAVE and debug flag and RA type.
        async fn available_enclave_keys(
            &self,
            request: tonic::Request<super::QueryAvailableEnclaveKeysRequest>,
        ) -> Result<
            tonic::Response<super::QueryAvailableEnclaveKeysResponse>,
            tonic::Status,
        >;
        /// Get the enclave key information for the specified enclave key address.
        async fn enclave_key(
            &self,
            request: tonic::Request<super::QueryEnclaveKeyRequest>,
        ) -> Result<tonic::Response<super::QueryEnclaveKeyResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct QueryServer<T: Query> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Query> QueryServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for QueryServer<T>
    where
        T: Query,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/lcp.service.enclave.v1.Query/EnclaveInfo" => {
                    #[allow(non_camel_case_types)]
                    struct EnclaveInfoSvc<T: Query>(pub Arc<T>);
                    impl<
                        T: Query,
                    > tonic::server::UnaryService<super::QueryEnclaveInfoRequest>
                    for EnclaveInfoSvc<T> {
                        type Response = super::QueryEnclaveInfoResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::QueryEnclaveInfoRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).enclave_info(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EnclaveInfoSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/lcp.service.enclave.v1.Query/AvailableEnclaveKeys" => {
                    #[allow(non_camel_case_types)]
                    struct AvailableEnclaveKeysSvc<T: Query>(pub Arc<T>);
                    impl<
                        T: Query,
                    > tonic::server::UnaryService<
                        super::QueryAvailableEnclaveKeysRequest,
                    > for AvailableEnclaveKeysSvc<T> {
                        type Response = super::QueryAvailableEnclaveKeysResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::QueryAvailableEnclaveKeysRequest,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).available_enclave_keys(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AvailableEnclaveKeysSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/lcp.service.enclave.v1.Query/EnclaveKey" => {
                    #[allow(non_camel_case_types)]
                    struct EnclaveKeySvc<T: Query>(pub Arc<T>);
                    impl<
                        T: Query,
                    > tonic::server::UnaryService<super::QueryEnclaveKeyRequest>
                    for EnclaveKeySvc<T> {
                        type Response = super::QueryEnclaveKeyResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::QueryEnclaveKeyRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).enclave_key(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = EnclaveKeySvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Query> Clone for QueryServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Query> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Query> tonic::server::NamedService for QueryServer<T> {
        const NAME: &'static str = "lcp.service.enclave.v1.Query";
    }
}
