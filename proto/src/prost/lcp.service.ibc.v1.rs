#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerificationResponse {
    #[prost(bytes = "vec", tag = "1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyClient {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub counterparty_client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "4")]
    pub expected_any_client_state: ::core::option::Option<
        super::super::super::super::google::protobuf::Any,
    >,
    #[prost(message, optional, tag = "5")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "6")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyClientConsensus {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub counterparty_client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "4")]
    pub consensus_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(message, optional, tag = "5")]
    pub expected_any_client_consensus_state: ::core::option::Option<
        super::super::super::super::google::protobuf::Any,
    >,
    #[prost(message, optional, tag = "6")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "7")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyConnection {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub connection_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "4")]
    pub expected_connection: ::core::option::Option<
        super::super::super::super::ibc::core::connection::v1::ConnectionEnd,
    >,
    #[prost(message, optional, tag = "5")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "6")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyChannel {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub port_id: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub channel_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "5")]
    pub expected_channel: ::core::option::Option<
        super::super::super::super::ibc::core::channel::v1::Channel,
    >,
    #[prost(message, optional, tag = "6")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "7")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyPacket {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub port_id: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub channel_id: ::prost::alloc::string::String,
    #[prost(uint64, tag = "5")]
    pub sequence: u64,
    #[prost(bytes = "vec", tag = "6")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "7")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "8")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyPacketAcknowledgement {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub port_id: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub channel_id: ::prost::alloc::string::String,
    #[prost(uint64, tag = "5")]
    pub sequence: u64,
    #[prost(bytes = "vec", tag = "6")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag = "7")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "8")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyPacketReceiptAbsense {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub port_id: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub channel_id: ::prost::alloc::string::String,
    #[prost(uint64, tag = "5")]
    pub sequence: u64,
    #[prost(message, optional, tag = "6")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "7")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyNextSequenceRecv {
    #[prost(string, tag = "1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes = "vec", tag = "2")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag = "3")]
    pub port_id: ::prost::alloc::string::String,
    #[prost(string, tag = "4")]
    pub channel_id: ::prost::alloc::string::String,
    #[prost(uint64, tag = "5")]
    pub next_sequence_recv: u64,
    #[prost(message, optional, tag = "6")]
    pub proof_height: ::core::option::Option<
        super::super::super::super::ibc::core::client::v1::Height,
    >,
    #[prost(bytes = "vec", tag = "7")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod msg_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Msg defines the ELC Msg service.
    #[derive(Debug, Clone)]
    pub struct MsgClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl MsgClient<tonic::transport::Channel> {
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
    impl<T> MsgClient<T>
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
        ) -> MsgClient<InterceptedService<T, F>>
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
            MsgClient::new(InterceptedService::new(inner, interceptor))
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
        /// VerifyClient defines a rpc handler method for MsgVerifyClient
        pub async fn verify_client(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyClient>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyClient",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyClientConsensus defines a rpc handler method for MsgVerifyClientConsensus
        pub async fn verify_client_consensus(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyClientConsensus>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyClientConsensus",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyConnection defines a rpc handler method for MsgVerifyConnection
        pub async fn verify_connection(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyConnection>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyConnection",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyChannel defines a rpc handler method for MsgVerifyChannel
        pub async fn verify_channel(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyChannel>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyChannel",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyPacket defines a rpc handler method for MsgVerifyPacket
        pub async fn verify_packet(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyPacket>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyPacket",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyPacketAcknowledgement defines a rpc handler method for MsgVerifyPacketAcknowledgement
        pub async fn verify_packet_acknowledgement(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyPacketAcknowledgement>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyPacketAcknowledgement",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyPacketReceiptAbsense defines a rpc handler method for MsgVerifyPacketReceiptAbsense
        pub async fn verify_packet_receipt_absense(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyPacketReceiptAbsense>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyPacketReceiptAbsense",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// VerifyNextSequenceRecv defines a rpc handler method for MsgVerifyNextSequenceRecv
        pub async fn verify_next_sequence_recv(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyNextSequenceRecv>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status> {
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
                "/lcp.service.ibc.v1.Msg/VerifyNextSequenceRecv",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
#[cfg(feature = "server")]
pub mod msg_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with MsgServer.
    #[async_trait]
    pub trait Msg: Send + Sync + 'static {
        /// VerifyClient defines a rpc handler method for MsgVerifyClient
        async fn verify_client(
            &self,
            request: tonic::Request<super::MsgVerifyClient>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyClientConsensus defines a rpc handler method for MsgVerifyClientConsensus
        async fn verify_client_consensus(
            &self,
            request: tonic::Request<super::MsgVerifyClientConsensus>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyConnection defines a rpc handler method for MsgVerifyConnection
        async fn verify_connection(
            &self,
            request: tonic::Request<super::MsgVerifyConnection>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyChannel defines a rpc handler method for MsgVerifyChannel
        async fn verify_channel(
            &self,
            request: tonic::Request<super::MsgVerifyChannel>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyPacket defines a rpc handler method for MsgVerifyPacket
        async fn verify_packet(
            &self,
            request: tonic::Request<super::MsgVerifyPacket>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyPacketAcknowledgement defines a rpc handler method for MsgVerifyPacketAcknowledgement
        async fn verify_packet_acknowledgement(
            &self,
            request: tonic::Request<super::MsgVerifyPacketAcknowledgement>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyPacketReceiptAbsense defines a rpc handler method for MsgVerifyPacketReceiptAbsense
        async fn verify_packet_receipt_absense(
            &self,
            request: tonic::Request<super::MsgVerifyPacketReceiptAbsense>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
        /// VerifyNextSequenceRecv defines a rpc handler method for MsgVerifyNextSequenceRecv
        async fn verify_next_sequence_recv(
            &self,
            request: tonic::Request<super::MsgVerifyNextSequenceRecv>,
        ) -> Result<tonic::Response<super::MsgVerificationResponse>, tonic::Status>;
    }
    /// Msg defines the ELC Msg service.
    #[derive(Debug)]
    pub struct MsgServer<T: Msg> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Msg> MsgServer<T> {
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
    impl<T, B> tonic::codegen::Service<http::Request<B>> for MsgServer<T>
    where
        T: Msg,
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
                "/lcp.service.ibc.v1.Msg/VerifyClient" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyClientSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyClient>
                    for VerifyClientSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyClient>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_client(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyClientSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyClientConsensus" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyClientConsensusSvc<T: Msg>(pub Arc<T>);
                    impl<
                        T: Msg,
                    > tonic::server::UnaryService<super::MsgVerifyClientConsensus>
                    for VerifyClientConsensusSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyClientConsensus>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_client_consensus(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyClientConsensusSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyConnection" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyConnectionSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyConnection>
                    for VerifyConnectionSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyConnection>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_connection(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyConnectionSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyChannel" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyChannelSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyChannel>
                    for VerifyChannelSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyChannel>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_channel(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyChannelSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyPacket" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyPacketSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyPacket>
                    for VerifyPacketSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyPacket>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_packet(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyPacketSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyPacketAcknowledgement" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyPacketAcknowledgementSvc<T: Msg>(pub Arc<T>);
                    impl<
                        T: Msg,
                    > tonic::server::UnaryService<super::MsgVerifyPacketAcknowledgement>
                    for VerifyPacketAcknowledgementSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<
                                super::MsgVerifyPacketAcknowledgement,
                            >,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_packet_acknowledgement(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyPacketAcknowledgementSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyPacketReceiptAbsense" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyPacketReceiptAbsenseSvc<T: Msg>(pub Arc<T>);
                    impl<
                        T: Msg,
                    > tonic::server::UnaryService<super::MsgVerifyPacketReceiptAbsense>
                    for VerifyPacketReceiptAbsenseSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyPacketReceiptAbsense>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_packet_receipt_absense(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyPacketReceiptAbsenseSvc(inner);
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
                "/lcp.service.ibc.v1.Msg/VerifyNextSequenceRecv" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyNextSequenceRecvSvc<T: Msg>(pub Arc<T>);
                    impl<
                        T: Msg,
                    > tonic::server::UnaryService<super::MsgVerifyNextSequenceRecv>
                    for VerifyNextSequenceRecvSvc<T> {
                        type Response = super::MsgVerificationResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgVerifyNextSequenceRecv>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).verify_next_sequence_recv(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = VerifyNextSequenceRecvSvc(inner);
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
    impl<T: Msg> Clone for MsgServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: Msg> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Msg> tonic::server::NamedService for MsgServer<T> {
        const NAME: &'static str = "lcp.service.ibc.v1.Msg";
    }
}
