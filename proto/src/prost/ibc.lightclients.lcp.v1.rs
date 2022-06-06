#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Header {
    #[prost(bytes = "vec", tag = "1")]
    pub update_client_commitment: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientState {
    #[prost(message, optional, tag = "1")]
    pub latest_height: ::core::option::Option<super::super::super::core::client::v1::Height>,
    #[prost(bytes = "vec", tag = "2")]
    pub mrenclave: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub key_expiration: u64,
    #[prost(bytes = "vec", repeated, tag = "4")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConsensusState {
    #[prost(bytes = "vec", tag = "1")]
    pub state_id: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub timestamp: u64,
}
