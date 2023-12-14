use crate::message::EmittedState;
use crate::prelude::*;
use crate::Error;
use lcp_types::Any;
use lcp_types::Height;
use prost::Message;

pub trait EthABIEncoder {
    fn ethabi_encode(self) -> Vec<u8>;
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;
}

/// the height is encoded as a tuple of 2 elements: (u64, u64)
pub(crate) struct EthABIHeight(ethabi::Uint, ethabi::Uint);

impl EthABIHeight {
    pub fn is_zero(&self) -> bool {
        self.0 == 0.into() && self.1 == 0.into()
    }
}

impl From<EthABIHeight> for Height {
    fn from(value: EthABIHeight) -> Self {
        Height::new(value.0.as_u64(), value.1.as_u64())
    }
}

impl From<EthABIHeight> for Option<Height> {
    fn from(value: EthABIHeight) -> Self {
        if value.is_zero() {
            None
        } else {
            Some(value.into())
        }
    }
}

impl From<EthABIHeight> for Vec<ethabi::Token> {
    fn from(value: EthABIHeight) -> Self {
        use ethabi::Token;
        vec![Token::Uint(value.0), Token::Uint(value.1)]
    }
}

impl From<Height> for EthABIHeight {
    fn from(value: Height) -> Self {
        Self(
            value.revision_number().into(),
            value.revision_height().into(),
        )
    }
}

impl From<Option<Height>> for EthABIHeight {
    fn from(value: Option<Height>) -> Self {
        value.unwrap_or_default().into()
    }
}

impl TryFrom<Vec<ethabi::Token>> for EthABIHeight {
    type Error = Error;
    fn try_from(value: Vec<ethabi::Token>) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(Error::invalid_abi(format!(
                "invalid height tuple length: {}",
                value.len()
            )));
        }
        let mut values = value.into_iter();
        let revision_number = values.next().unwrap().into_uint().unwrap();
        let revision_height = values.next().unwrap().into_uint().unwrap();
        Ok(Self(revision_number, revision_height))
    }
}

/// the height is encoded as a tuple of 2 elements: ((u64, u64), bytes)
pub(crate) struct EthABIEmittedState(EthABIHeight, ethabi::Bytes);

impl From<EmittedState> for EthABIEmittedState {
    fn from(value: EmittedState) -> Self {
        Self(value.0.into(), value.1.encode_to_vec())
    }
}

impl TryFrom<EthABIEmittedState> for EmittedState {
    type Error = Error;
    fn try_from(value: EthABIEmittedState) -> Result<Self, Self::Error> {
        Ok(Self(value.0.into(), Any::try_from(value.1)?))
    }
}

impl From<EthABIEmittedState> for Vec<ethabi::Token> {
    fn from(value: EthABIEmittedState) -> Self {
        use ethabi::Token;
        vec![Token::Tuple(value.0.into()), Token::Bytes(value.1)]
    }
}

impl TryFrom<Vec<ethabi::Token>> for EthABIEmittedState {
    type Error = Error;
    fn try_from(value: Vec<ethabi::Token>) -> Result<Self, Self::Error> {
        if value.len() != 2 {
            return Err(Error::invalid_abi(format!(
                "invalid emitted state tuple length: {}",
                value.len()
            )));
        }
        let mut values = value.into_iter();
        let height = values.next().unwrap().into_tuple().unwrap().try_into()?;
        let state_id = values.next().unwrap().into_bytes().unwrap();
        Ok(Self(height, state_id))
    }
}
