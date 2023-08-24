use crate::{prelude::*, TypeError};
use core::{
    fmt::{Display, Error as FmtError, Formatter},
    str::FromStr,
};

/// Path separator (ie. forward slash '/')
const PATH_SEPARATOR: char = '/';
const VALID_SPECIAL_CHARS: &str = "._+-#[]<>";

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
        validate_client_identifier(s)?;
        Ok(Self(s.to_string()))
    }
}

impl PartialEq<str> for ClientId {
    fn eq(&self, other: &str) -> bool {
        self.as_str().eq(other)
    }
}

#[cfg(feature = "ibc")]
impl From<ClientId> for ibc::core::ics24_host::identifier::ClientId {
    fn from(value: ClientId) -> Self {
        Self::from_str(value.as_str()).unwrap()
    }
}

#[cfg(feature = "ibc")]
impl From<ibc::core::ics24_host::identifier::ClientId> for ClientId {
    fn from(value: ibc::core::ics24_host::identifier::ClientId) -> Self {
        Self::from_str(value.as_str()).unwrap()
    }
}

/// Default validator function for Client identifiers.
///
/// A valid identifier must be between 9-64 characters and only contain lowercase
/// alphabetic characters,
fn validate_client_identifier(id: &str) -> Result<(), TypeError> {
    validate_identifier(id, 9, 64)
}

/// Default validator function for identifiers.
///
/// A valid identifier only contain lowercase alphabetic characters, and be of a given min and max
/// length.
fn validate_identifier(id: &str, min: usize, max: usize) -> Result<(), TypeError> {
    assert!(max >= min);

    // Check identifier is not empty
    if id.is_empty() {
        return Err(TypeError::client_id_empty());
    }

    // Check identifier does not contain path separators
    if id.contains(PATH_SEPARATOR) {
        return Err(TypeError::client_id_contain_separator(id.to_owned()));
    }

    // Check identifier length is between given min/max
    if id.len() < min || id.len() > max {
        return Err(TypeError::client_id_invalid_length(
            id.to_owned(),
            id.len(),
            min,
            max,
        ));
    }

    // Check that the identifier comprises only valid characters:
    // - Alphanumeric
    // - `.`, `_`, `+`, `-`, `#`
    // - `[`, `]`, `<`, `>`
    if !id
        .chars()
        .all(|c| c.is_alphanumeric() || VALID_SPECIAL_CHARS.contains(c))
    {
        return Err(TypeError::client_id_invalid_character(id.to_owned()));
    }

    // All good!
    Ok(())
}
