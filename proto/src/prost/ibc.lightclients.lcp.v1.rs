#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateClientMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub proxy_message: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub signatures: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RegisterEnclaveKeyMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub report: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub signature: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub signing_cert: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub operator_signature: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ZkdcapRegisterEnclaveKeyMessage {
    #[prost(bytes = "vec", tag = "1")]
    pub commit: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub operator_signature: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UpdateOperatorsMessage {
    #[prost(uint64, tag = "1")]
    pub nonce: u64,
    #[prost(bytes = "vec", repeated, tag = "2")]
    pub new_operators: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(uint64, tag = "3")]
    pub new_operators_threshold_numerator: u64,
    #[prost(uint64, tag = "4")]
    pub new_operators_threshold_denominator: u64,
    #[prost(bytes = "vec", repeated, tag = "5")]
    pub signatures: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClientState {
    #[prost(bytes = "vec", tag = "1")]
    pub mrenclave: ::prost::alloc::vec::Vec<u8>,
    #[prost(uint64, tag = "2")]
    pub key_expiration: u64,
    #[prost(bool, tag = "3")]
    pub frozen: bool,
    #[prost(message, optional, tag = "4")]
    pub latest_height: ::core::option::Option<
        super::super::super::core::client::v1::Height,
    >,
    /// e.g. SW_HARDENING_NEEDED, CONFIGURATION_AND_SW_HARDENING_NEEDED (except "OK")
    #[prost(string, repeated, tag = "5")]
    pub allowed_quote_statuses: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// e.g. INTEL-SA-XXXXX
    #[prost(string, repeated, tag = "6")]
    pub allowed_advisory_ids: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(bytes = "vec", repeated, tag = "7")]
    pub operators: ::prost::alloc::vec::Vec<::prost::alloc::vec::Vec<u8>>,
    #[prost(uint64, tag = "8")]
    pub operators_nonce: u64,
    #[prost(uint64, tag = "9")]
    pub operators_threshold_numerator: u64,
    #[prost(uint64, tag = "10")]
    pub operators_threshold_denominator: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConsensusState {
    #[prost(bytes = "vec", tag = "1")]
    pub state_id: ::prost::alloc::vec::Vec<u8>,
    /// unix timestamp in seconds
    #[prost(uint64, tag = "2")]
    pub timestamp: u64,
}
