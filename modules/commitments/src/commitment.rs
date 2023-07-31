use crate::prelude::*;
use crate::{Error, StateID};
use core::fmt::Display;
use lcp_types::{Any, Height, Time};
use prost::Message;
use serde::{Deserialize, Serialize};
use validation_context::ValidationParams;

pub const COMMITMENT_SCHEMA_VERSION: u64 = 1;
pub const COMMITMENT_TYPE_UPDATE_CLIENT: u64 = 1;
pub const COMMITMENT_TYPE_STATE: u64 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Commitment {
    UpdateClient(UpdateClientCommitment),
    State(StateCommitment),
}

impl Display for Commitment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Commitment::UpdateClient(c) => write!(f, "UpdateClient({})", c),
            Commitment::State(c) => write!(f, "State({})", c),
        }
    }
}

impl TryFrom<Commitment> for UpdateClientCommitment {
    type Error = Error;
    fn try_from(value: Commitment) -> Result<Self, Self::Error> {
        match value {
            Commitment::UpdateClient(c) => Ok(c),
            _ => Err(Error::unexpected_commitment_type(
                COMMITMENT_TYPE_UPDATE_CLIENT,
                value.commitment_type(),
            )),
        }
    }
}

impl TryFrom<Commitment> for StateCommitment {
    type Error = Error;
    fn try_from(value: Commitment) -> Result<Self, Self::Error> {
        match value {
            Commitment::State(c) => Ok(c),
            _ => Err(Error::unexpected_commitment_type(
                COMMITMENT_TYPE_STATE,
                value.commitment_type(),
            )),
        }
    }
}

impl Commitment {
    pub fn to_commitment_bytes(self) -> Vec<u8> {
        EthABIHeaderedCommitment {
            header: self.header().as_ref().try_into().unwrap(),
            commitment: match self {
                Commitment::UpdateClient(c) => c.to_vec(),
                Commitment::State(c) => c.to_vec(),
            },
        }
        .encode()
    }

    pub fn from_commitment_bytes(bz: &[u8]) -> Result<Self, Error> {
        let eth_abi_commitment = EthABIHeaderedCommitment::decode(bz)?;
        let (version, commitment_type) = {
            let header = eth_abi_commitment.header;
            let mut version = [0u8; 8];
            version.copy_from_slice(&header[0..8]);
            let mut commitment_type = [0u8; 8];
            commitment_type.copy_from_slice(&header[8..16]);
            (
                u64::from_be_bytes(version),
                u64::from_be_bytes(commitment_type),
            )
        };
        assert!(version == COMMITMENT_SCHEMA_VERSION);
        let commitment = eth_abi_commitment.commitment;
        match commitment_type {
            COMMITMENT_TYPE_UPDATE_CLIENT => Ok(Commitment::UpdateClient(
                EthABIUpdateClientCommitment::decode(&commitment)?.try_into()?,
            )),
            COMMITMENT_TYPE_STATE => Ok(Commitment::State(
                EthABIStateCommitment::decode(&commitment)?.try_into()?,
            )),
            _ => Err(Error::invalid_abi(format!(
                "invalid commitment type: {}",
                commitment_type
            ))),
        }
    }

    // msb 8 bytes: version
    // next 8 bytes: commitment type
    pub fn header(&self) -> [u8; 32] {
        let mut header = [0u8; 32];
        header[0..8].copy_from_slice(&COMMITMENT_SCHEMA_VERSION.to_be_bytes());
        header[8..16].copy_from_slice(&self.commitment_type().to_be_bytes());
        header
    }

    pub fn commitment_type(&self) -> u64 {
        match self {
            Commitment::UpdateClient(_) => COMMITMENT_TYPE_UPDATE_CLIENT,
            Commitment::State(_) => COMMITMENT_TYPE_STATE,
        }
    }
}

pub(crate) struct EthABIHeaderedCommitment {
    header: ethabi::FixedBytes, // bytes32
    commitment: ethabi::Bytes,  // bytes
}

impl EthABIHeaderedCommitment {
    pub fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::FixedBytes(self.header),
            Token::Bytes(self.commitment),
        ])])
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::FixedBytes(32),
                ParamType::Bytes,
            ])],
            bytes,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 2
        assert!(tuple.len() == 2);
        let mut values = tuple.into_iter();
        Ok(Self {
            header: values.next().unwrap().into_fixed_bytes().unwrap(),
            commitment: values.next().unwrap().into_bytes().unwrap(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateClientCommitment {
    pub prev_state_id: Option<StateID>,
    pub new_state_id: StateID,
    pub new_state: Option<Any>,
    pub prev_height: Option<Height>,
    pub new_height: Height,
    pub timestamp: Time,
    pub validation_params: ValidationParams,
}

impl From<UpdateClientCommitment> for Commitment {
    fn from(value: UpdateClientCommitment) -> Self {
        Self::UpdateClient(value)
    }
}

impl Default for UpdateClientCommitment {
    fn default() -> Self {
        UpdateClientCommitment {
            timestamp: Time::unix_epoch(),
            prev_state_id: Default::default(),
            new_state_id: Default::default(),
            new_state: Default::default(),
            prev_height: Default::default(),
            new_height: Default::default(),
            validation_params: Default::default(),
        }
    }
}

impl Display for UpdateClientCommitment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "prev_state_id={} new_state_id={} new_state_include={} prev_height={:?} new_height={:?} timestamp={} validation_params={{{}}}",
            self.prev_state_id.map_or("".to_string(), |s| s.to_string()), self.new_state_id, self.new_state.is_some(), self.prev_height.map_or("".to_string(), |h| h.to_string()), self.new_height.to_string(), self.timestamp, self.validation_params
        )
    }
}

pub(crate) struct EthABIUpdateClientCommitment {
    prev_state_id: ethabi::FixedBytes, // bytes32
    new_state_id: ethabi::FixedBytes,  // bytes32
    new_state: ethabi::Bytes,          // bytes
    prev_height: EthABIHeight,         // (u64, u64)
    new_height: EthABIHeight,          // (u64, u64)
    timestamp: ethabi::Uint,           // u128
    validation_params: ethabi::Bytes,  // bytes
}

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

impl EthABIUpdateClientCommitment {
    pub fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::FixedBytes(self.prev_state_id),
            Token::FixedBytes(self.new_state_id),
            Token::Bytes(self.new_state),
            Token::Tuple(self.prev_height.into()),
            Token::Tuple(self.new_height.into()),
            Token::Uint(self.timestamp),
            Token::Bytes(self.validation_params),
        ])])
    }

    pub fn decode(bz: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::FixedBytes(32),
                ParamType::FixedBytes(32),
                ParamType::Bytes,
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::Uint(64),
                ParamType::Bytes,
            ])],
            bz,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 7
        assert!(tuple.len() == 7);
        let mut values = tuple.into_iter();
        Ok(Self {
            prev_state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
            new_state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
            new_state: values.next().unwrap().into_bytes().unwrap(),
            prev_height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            new_height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            timestamp: values.next().unwrap().into_uint().unwrap(),
            validation_params: values.next().unwrap().into_bytes().unwrap(),
        })
    }
}

impl From<UpdateClientCommitment> for EthABIUpdateClientCommitment {
    fn from(value: UpdateClientCommitment) -> Self {
        use ethabi::*;
        Self {
            prev_state_id: FixedBytes::from(
                value.prev_state_id.unwrap_or_default().to_vec().as_slice(),
            ),
            new_state_id: FixedBytes::from(value.new_state_id.to_vec().as_slice()),
            new_state: value
                .new_state
                .map_or(Bytes::default(), |s| s.encode_to_vec()),
            prev_height: value.prev_height.into(),
            new_height: value.new_height.into(),
            timestamp: Uint::from(value.timestamp.as_unix_timestamp_nanos()),
            validation_params: value.validation_params.to_vec(),
        }
    }
}

impl TryFrom<EthABIUpdateClientCommitment> for UpdateClientCommitment {
    type Error = Error;
    fn try_from(value: EthABIUpdateClientCommitment) -> Result<Self, Self::Error> {
        Ok(Self {
            prev_state_id: bytes_to_bytes32(value.prev_state_id)?.map(StateID::from_bytes_array),
            new_state_id: value.new_state_id.as_slice().try_into()?,
            new_state: if value.new_state.is_empty() {
                None
            } else {
                Some(Any::try_from(value.new_state)?)
            },
            prev_height: value.prev_height.into(),
            new_height: value.new_height.into(),
            timestamp: Time::from_unix_timestamp_nanos(value.timestamp.as_u128())?,
            validation_params: ValidationParams::from_bytes(value.validation_params.as_slice()),
        })
    }
}

fn bytes_to_bytes32(bytes: Vec<u8>) -> Result<Option<[u8; 32]>, Error> {
    if bytes == [0u8; 32] {
        Ok(None)
    } else if bytes.len() == 32 {
        // SAFETY: the length of bytes is 32
        Ok(Some(bytes.as_slice().try_into().unwrap()))
    } else {
        Err(Error::invalid_optional_bytes_length(32, bytes.len()))
    }
}

impl UpdateClientCommitment {
    pub fn to_vec(self) -> Vec<u8> {
        Into::<EthABIUpdateClientCommitment>::into(self).encode()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        EthABIUpdateClientCommitment::decode(bz).and_then(|v| v.try_into())
    }
}

pub type CommitmentPrefix = Vec<u8>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StateCommitment {
    pub prefix: CommitmentPrefix,
    pub path: String,
    pub value: Option<[u8; 32]>,
    pub height: Height,
    pub state_id: StateID,
}

impl From<StateCommitment> for Commitment {
    fn from(value: StateCommitment) -> Self {
        Self::State(value)
    }
}

pub(crate) struct EthABIStateCommitment {
    prefix: ethabi::Bytes,        // bytes
    path: ethabi::Bytes,          // bytes
    value: ethabi::FixedBytes,    // bytes32
    height: EthABIHeight,         // (uint64, uint64)
    state_id: ethabi::FixedBytes, // bytes32
}

impl EthABIStateCommitment {
    pub fn encode(self) -> Vec<u8> {
        use ethabi::Token;
        ethabi::encode(&[Token::Tuple(vec![
            Token::Bytes(self.prefix),
            Token::Bytes(self.path),
            Token::FixedBytes(self.value),
            Token::Tuple(self.height.into()),
            Token::FixedBytes(self.state_id),
        ])])
    }

    pub fn decode(bz: &[u8]) -> Result<Self, Error> {
        use ethabi::ParamType;
        let tuple = ethabi::decode(
            &[ParamType::Tuple(vec![
                ParamType::Bytes,
                ParamType::Bytes,
                ParamType::FixedBytes(32),
                ParamType::Tuple(vec![ParamType::Uint(64), ParamType::Uint(64)]),
                ParamType::FixedBytes(32),
            ])],
            bz,
        )?
        .into_iter()
        .next()
        .unwrap()
        .into_tuple()
        .unwrap();

        // if the decoding is successful, the length of the tuple should be 5
        assert!(tuple.len() == 5);
        let mut values = tuple.into_iter();
        Ok(Self {
            prefix: values.next().unwrap().into_bytes().unwrap(),
            path: values.next().unwrap().into_bytes().unwrap().to_vec(),
            value: values.next().unwrap().into_fixed_bytes().unwrap(),
            height: values.next().unwrap().into_tuple().unwrap().try_into()?,
            state_id: values.next().unwrap().into_fixed_bytes().unwrap(),
        })
    }
}

impl From<StateCommitment> for EthABIStateCommitment {
    fn from(value: StateCommitment) -> Self {
        use ethabi::*;
        Self {
            prefix: value.prefix,
            path: Bytes::from(value.path),
            value: FixedBytes::from(value.value.unwrap_or_default()),
            height: EthABIHeight::from(value.height),
            state_id: value.state_id.to_vec(),
        }
    }
}

impl TryFrom<EthABIStateCommitment> for StateCommitment {
    type Error = Error;
    fn try_from(value: EthABIStateCommitment) -> Result<Self, Self::Error> {
        Ok(Self {
            prefix: value.prefix,
            path: String::from_utf8(value.path)?,
            value: bytes_to_bytes32(value.value)?,
            height: value.height.into(),
            state_id: value.state_id.as_slice().try_into()?,
        })
    }
}

impl Display for StateCommitment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "prefix={:?} path={} value={:?} height={} state_id={}",
            self.prefix, self.path, self.value, self.height, self.state_id
        )
    }
}

impl StateCommitment {
    pub fn new(
        prefix: CommitmentPrefix,
        path: String,
        value: Option<[u8; 32]>,
        height: Height,
        state_id: StateID,
    ) -> Self {
        Self {
            prefix,
            path,
            value,
            height,
            state_id,
        }
    }

    pub fn to_vec(self) -> Vec<u8> {
        Into::<EthABIStateCommitment>::into(self).encode()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        EthABIStateCommitment::decode(bz).and_then(|v| v.try_into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibc::{
        clients::ics07_tendermint::client_type,
        core::ics24_host::{identifier::ClientId, path::Path},
    };
    use prost_types::Any as ProtoAny;
    use rand::{distributions::Uniform, thread_rng, Rng};

    #[test]
    fn test_update_client_commitment_converter() {
        for _ in 0..1024 {
            let c1 = UpdateClientCommitment {
                prev_state_id: rand_or_none(gen_rand_state_id),
                new_state_id: gen_rand_state_id(),
                new_state: rand_or_none(|| -> Any {
                    ProtoAny {
                        type_url: "/".to_owned(),
                        value: gen_rand_vec(64),
                    }
                    .try_into()
                    .unwrap()
                }),
                prev_height: rand_or_none(gen_rand_height),
                new_height: gen_rand_height(),
                timestamp: Time::now(),
                validation_params: Default::default(),
            };
            let v = c1.clone().to_vec();
            let c2 = UpdateClientCommitment::from_bytes(&v).unwrap();
            assert_eq!(c1, c2);
        }
    }

    #[test]
    fn test_state_commitment_converter() {
        for _ in 0..256 {
            let c1 = StateCommitment {
                prefix: "ibc".as_bytes().to_vec(),
                path: Path::ClientType(ibc::core::ics24_host::path::ClientTypePath(
                    ClientId::new(client_type(), thread_rng().gen()).unwrap(),
                ))
                .to_string(),
                value: rand_or_none(|| gen_rand_vec(32).as_slice().try_into().unwrap()),
                height: gen_rand_height(),
                state_id: gen_rand_state_id(),
            };
            let v = c1.clone().to_vec();
            let c2 = StateCommitment::from_bytes(&v).unwrap();
            assert_eq!(c1, c2);
        }
    }

    fn gen_rand_vec(size: usize) -> Vec<u8> {
        let mut rng = thread_rng();
        let range = Uniform::new(0, u8::MAX);
        let vals: Vec<u8> = (0..size).map(|_| rng.sample(range)).collect();
        vals
    }

    fn gen_rand_state_id() -> StateID {
        gen_rand_vec(32).as_slice().try_into().unwrap()
    }

    fn gen_rand_height() -> Height {
        Height::new(thread_rng().gen(), thread_rng().gen())
    }

    fn rand_or_none<T, F: Fn() -> T>(func: F) -> Option<T> {
        if thread_rng().gen_bool(0.5) {
            Some(func())
        } else {
            None
        }
    }
}
