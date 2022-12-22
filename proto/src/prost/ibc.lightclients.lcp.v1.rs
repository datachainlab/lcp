#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateClientHeader {
    #[prost(bytes="vec", tag="1")]
    pub commitment: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="2")]
    pub signer: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegisterEnclaveKeyHeader {
    #[prost(string, tag="1")]
    pub report: ::prost::alloc::string::String,
    #[prost(bytes="vec", tag="2")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes="vec", tag="3")]
    pub signing_cert: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientState {
    #[prost(message, optional, tag="1")]
    pub latest_height: ::core::option::Option<super::super::super::core::client::v1::Height>,
    #[prost(bytes="vec", tag="2")]
    pub mrenclave: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag="3")]
    pub key_expiration: u64,
    #[prost(bytes="vec", repeated, tag="4")]
    pub keys: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    /// unix timestamp in seconds
    #[prost(uint64, repeated, tag="5")]
    pub attestation_times: ::prost::alloc::vec::Vec<u64>,
    /// e.g. SW_HARDENING_NEEDED, CONFIGURATION_AND_SW_HARDENING_NEEDED (except "OK")
    #[prost(string, repeated, tag="6")]
    pub allowed_quote_statuses: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// e.g. INTEL-SA-XXXXX
    #[prost(string, repeated, tag="7")]
    pub allowed_advisory_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConsensusState {
    #[prost(bytes="vec", tag="1")]
    pub state_id: ::prost::alloc::vec::Vec<u8>,
    /// unix timestamp in seconds
    #[prost(uint64, tag="2")]
    pub timestamp: u64,
}
