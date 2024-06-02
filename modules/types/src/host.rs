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

    /// Validate the client identifier
    pub fn validate(&self, client_type: &str) -> Result<(), TypeError> {
        validate_client_identifier(self.0.as_str())?;
        self.0.rfind('-').map_or(
            Err(TypeError::client_id_invalid_format(self.0.clone())),
            |pos| {
                let (client_type_, prefixed_counter) = self.0.split_at(pos);
                if client_type_ != client_type {
                    return Err(TypeError::client_id_invalid_client_type(
                        self.0.clone(),
                        client_type.to_string(),
                    ));
                }
                match prefixed_counter.strip_prefix('-') {
                    None => Err(TypeError::client_id_invalid_counter(self.0.clone())),
                    Some(counter) if counter.starts_with('0') && counter.len() > 1 => {
                        Err(TypeError::client_id_invalid_counter(self.0.clone()))
                    }
                    Some(counter) => {
                        counter.parse::<u64>().map_err(|e| {
                            TypeError::client_id_invalid_counter_parse_int_error(self.0.clone(), e)
                        })?;
                        Ok(())
                    }
                }
            },
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_client_id() {
        let cases = vec![
            ("testclient", "testclient-0"),
            ("testclient", "testclient-1"),
            ("testclient", "testclient-10"),
            ("07-tendermint", "07-tendermint-0"),
            ("07-tendermint", "07-tendermint-1"),
            ("07-tendermint", "07-tendermint-10"),
            ("testclient", "testclient-12345678901234567890"),
        ];
        for (i, (client_type, c)) in cases.iter().enumerate() {
            let cl = ClientId::from_str(c).unwrap();
            let res = cl.validate(client_type);
            assert!(res.is_ok(), "case: {}, error: {:?}", i, res);
        }
    }

    #[test]
    fn test_invalid_client_id() {
        let cases = vec![
            ("testclient", "testclient"),
            ("testclient", "testclient1"),
            ("07-tendermint", "07-tendermint"),
            ("07-tendermint", "07-tendermint0"),
            ("07-tendermint", "07-tendermint1"),
            ("07-tendermint", "07-tendermint-01"),
            ("client", "client-0"),
            ("", ""),
            ("", "07-tendermint"),
            ("testclient", "testclient-123456789012345678901"),
        ];
        for (i, (client_type, c)) in cases.iter().enumerate() {
            let res = ClientId::from_str(c).and_then(|cl| cl.validate(client_type));
            assert!(res.is_err(), "case: {}, error: {:?}", i, res);
        }
    }
}
