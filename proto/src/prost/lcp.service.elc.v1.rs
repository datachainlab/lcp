/// MsgCreateClient defines a message to create an IBC client
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgCreateClient {
    /// light client state
    #[prost(message, optional, tag="1")]
    pub client_state: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
    /// consensus state associated with the client that corresponds to a given
    /// height.
    #[prost(message, optional, tag="2")]
    pub consensus_state: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
    /// signer address
    #[prost(string, tag="3")]
    pub signer: ::prost::alloc::string::String,
}
/// MsgCreateClientResponse defines the Msg/CreateClient response type.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgCreateClientResponse {
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="4")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
/// MsgUpdateClient defines an sdk.Msg to update a IBC client state using
/// the given header.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgUpdateClient {
    /// client unique identifier
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
    /// header to update the light client
    #[prost(message, optional, tag="2")]
    pub header: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
}
/// MsgUpdateClientResponse defines the Msg/UpdateClient response type.
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgUpdateClientResponse {
    #[prost(bytes="vec", tag="1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyClient {
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub target_any_client_state: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
    #[prost(bytes="vec", tag="3")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="4")]
    pub counterparty_client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="5")]
    pub proof_height: ::core::option::Option<super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(bytes="vec", tag="6")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyClientResponse {
    #[prost(bytes="vec", tag="1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyClientConsensus {
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub target_any_client_consensus_state: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
    #[prost(bytes="vec", tag="3")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="4")]
    pub counterparty_client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="5")]
    pub counterparty_consensus_height: ::core::option::Option<super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(message, optional, tag="6")]
    pub proof_height: ::core::option::Option<super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(bytes="vec", tag="7")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyClientConsensusResponse {
    #[prost(bytes="vec", tag="1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyConnection {
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub expected_connection: ::core::option::Option<super::super::super::super::ibc::core::connection::v1::ConnectionEnd>,
    #[prost(bytes="vec", tag="3")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="4")]
    pub counterparty_connection_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="5")]
    pub proof_height: ::core::option::Option<super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(bytes="vec", tag="6")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyConnectionResponse {
    #[prost(bytes="vec", tag="1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyChannel {
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="2")]
    pub expected_channel: ::core::option::Option<super::super::super::super::ibc::core::channel::v1::Channel>,
    #[prost(bytes="vec", tag="3")]
    pub prefix: ::prost::alloc::vec::Vec<u8>,
    #[prost(string, tag="4")]
    pub counterparty_port_id: ::prost::alloc::string::String,
    #[prost(string, tag="5")]
    pub counterparty_channel_id: ::prost::alloc::string::String,
    #[prost(message, optional, tag="6")]
    pub proof_height: ::core::option::Option<super::super::super::super::ibc::core::client::v1::Height>,
    #[prost(bytes="vec", tag="7")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MsgVerifyChannelResponse {
    #[prost(bytes="vec", tag="1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod msg_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
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
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        /// CreateClient defines a rpc handler method for MsgCreateClient.
        pub async fn create_client(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgCreateClient>,
        ) -> Result<tonic::Response<super::MsgCreateClientResponse>, tonic::Status> {
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
                "/lcp.service.elc.v1.Msg/CreateClient",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// UpdateClient defines a rpc handler method for MsgUpdateClient.
        pub async fn update_client(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgUpdateClient>,
        ) -> Result<tonic::Response<super::MsgUpdateClientResponse>, tonic::Status> {
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
                "/lcp.service.elc.v1.Msg/UpdateClient",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn verify_client(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyClient>,
        ) -> Result<tonic::Response<super::MsgVerifyClientResponse>, tonic::Status> {
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
                "/lcp.service.elc.v1.Msg/VerifyClient",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn verify_client_consensus(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyClientConsensus>,
        ) -> Result<
            tonic::Response<super::MsgVerifyClientConsensusResponse>,
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
                "/lcp.service.elc.v1.Msg/VerifyClientConsensus",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn verify_connection(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyConnection>,
        ) -> Result<tonic::Response<super::MsgVerifyConnectionResponse>, tonic::Status> {
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
                "/lcp.service.elc.v1.Msg/VerifyConnection",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn verify_channel(
            &mut self,
            request: impl tonic::IntoRequest<super::MsgVerifyChannel>,
        ) -> Result<tonic::Response<super::MsgVerifyChannelResponse>, tonic::Status> {
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
                "/lcp.service.elc.v1.Msg/VerifyChannel",
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
    ///Generated trait containing gRPC methods that should be implemented for use with MsgServer.
    #[async_trait]
    pub trait Msg: Send + Sync + 'static {
        /// CreateClient defines a rpc handler method for MsgCreateClient.
        async fn create_client(
            &self,
            request: tonic::Request<super::MsgCreateClient>,
        ) -> Result<tonic::Response<super::MsgCreateClientResponse>, tonic::Status>;
        /// UpdateClient defines a rpc handler method for MsgUpdateClient.
        async fn update_client(
            &self,
            request: tonic::Request<super::MsgUpdateClient>,
        ) -> Result<tonic::Response<super::MsgUpdateClientResponse>, tonic::Status>;
        async fn verify_client(
            &self,
            request: tonic::Request<super::MsgVerifyClient>,
        ) -> Result<tonic::Response<super::MsgVerifyClientResponse>, tonic::Status>;
        async fn verify_client_consensus(
            &self,
            request: tonic::Request<super::MsgVerifyClientConsensus>,
        ) -> Result<
            tonic::Response<super::MsgVerifyClientConsensusResponse>,
            tonic::Status,
        >;
        async fn verify_connection(
            &self,
            request: tonic::Request<super::MsgVerifyConnection>,
        ) -> Result<tonic::Response<super::MsgVerifyConnectionResponse>, tonic::Status>;
        async fn verify_channel(
            &self,
            request: tonic::Request<super::MsgVerifyChannel>,
        ) -> Result<tonic::Response<super::MsgVerifyChannelResponse>, tonic::Status>;
    }
    /// Msg defines the ELC Msg service.
    #[derive(Debug)]
    pub struct MsgServer<T: Msg> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
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
                "/lcp.service.elc.v1.Msg/CreateClient" => {
                    #[allow(non_camel_case_types)]
                    struct CreateClientSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgCreateClient>
                    for CreateClientSvc<T> {
                        type Response = super::MsgCreateClientResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgCreateClient>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).create_client(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CreateClientSvc(inner);
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
                "/lcp.service.elc.v1.Msg/UpdateClient" => {
                    #[allow(non_camel_case_types)]
                    struct UpdateClientSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgUpdateClient>
                    for UpdateClientSvc<T> {
                        type Response = super::MsgUpdateClientResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MsgUpdateClient>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).update_client(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UpdateClientSvc(inner);
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
                "/lcp.service.elc.v1.Msg/VerifyClient" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyClientSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyClient>
                    for VerifyClientSvc<T> {
                        type Response = super::MsgVerifyClientResponse;
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
                "/lcp.service.elc.v1.Msg/VerifyClientConsensus" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyClientConsensusSvc<T: Msg>(pub Arc<T>);
                    impl<
                        T: Msg,
                    > tonic::server::UnaryService<super::MsgVerifyClientConsensus>
                    for VerifyClientConsensusSvc<T> {
                        type Response = super::MsgVerifyClientConsensusResponse;
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
                "/lcp.service.elc.v1.Msg/VerifyConnection" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyConnectionSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyConnection>
                    for VerifyConnectionSvc<T> {
                        type Response = super::MsgVerifyConnectionResponse;
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
                "/lcp.service.elc.v1.Msg/VerifyChannel" => {
                    #[allow(non_camel_case_types)]
                    struct VerifyChannelSvc<T: Msg>(pub Arc<T>);
                    impl<T: Msg> tonic::server::UnaryService<super::MsgVerifyChannel>
                    for VerifyChannelSvc<T> {
                        type Response = super::MsgVerifyChannelResponse;
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
    impl<T: Msg> tonic::transport::NamedService for MsgServer<T> {
        const NAME: &'static str = "lcp.service.elc.v1.Msg";
    }
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryClientRequest {
    #[prost(string, tag="1")]
    pub client_id: ::prost::alloc::string::String,
}
#[derive(::serde::Serialize, ::serde::Deserialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryClientResponse {
    /// light client state
    #[prost(message, optional, tag="1")]
    pub client_state: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
    /// consensus state associated with the client that corresponds to a given
    /// height.
    #[prost(message, optional, tag="2")]
    pub consensus_state: ::core::option::Option<super::super::super::super::google::protobuf::Any>,
}
/// Generated client implementations.
#[cfg(feature = "client")]
pub mod query_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Query defines the ELC Query service.
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
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn client(
            &mut self,
            request: impl tonic::IntoRequest<super::QueryClientRequest>,
        ) -> Result<tonic::Response<super::QueryClientResponse>, tonic::Status> {
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
                "/lcp.service.elc.v1.Query/Client",
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
    ///Generated trait containing gRPC methods that should be implemented for use with QueryServer.
    #[async_trait]
    pub trait Query: Send + Sync + 'static {
        async fn client(
            &self,
            request: tonic::Request<super::QueryClientRequest>,
        ) -> Result<tonic::Response<super::QueryClientResponse>, tonic::Status>;
    }
    /// Query defines the ELC Query service.
    #[derive(Debug)]
    pub struct QueryServer<T: Query> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
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
                "/lcp.service.elc.v1.Query/Client" => {
                    #[allow(non_camel_case_types)]
                    struct ClientSvc<T: Query>(pub Arc<T>);
                    impl<T: Query> tonic::server::UnaryService<super::QueryClientRequest>
                    for ClientSvc<T> {
                        type Response = super::QueryClientResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::QueryClientRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).client(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ClientSvc(inner);
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
    impl<T: Query> tonic::transport::NamedService for QueryServer<T> {
        const NAME: &'static str = "lcp.service.elc.v1.Query";
    }
}
