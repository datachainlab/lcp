use crate::{prelude::*, TypeError};
use core::{
    fmt::{Display, Error as FmtError, Formatter},
    str::FromStr,
};
use ibc::core::ics24_host::identifier::ClientId as IBCClientId;
use ibc::core::ics24_host::validate::validate_client_identifier;

/// ClientId is an identifier of Enclave Light Client(ELC)
#[derive(
    Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ClientId(String);

impl ClientId {
    pub fn new(client_type: &str, counter: u64) -> Result<Self, TypeError> {
        let id = format!("{client_type}-{counter}");
        Self::from_str(id.as_str())
    }

    /// Get this identifier as a borrowed `&str`
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get this identifier as a borrowed byte slice
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

/// This implementation provides a `to_string` method.
impl Display for ClientId {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), FmtError> {
        write!(f, "{}", self.0)
    }
}

impl FromStr for ClientId {
    type Err = TypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        validate_client_identifier(s)
            .map(|_| Self(s.to_string()))
            .map_err(|e| TypeError::invalid_client_id_format(s.to_string(), e.to_string()))
    }
}

impl PartialEq<str> for ClientId {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

impl From<ClientId> for IBCClientId {
    fn from(value: ClientId) -> Self {
        Self::from_str(value.as_str()).unwrap()
    }
}

impl From<IBCClientId> for ClientId {
    fn from(value: IBCClientId) -> Self {
        Self::from_str(value.as_str()).unwrap()
    }
}
