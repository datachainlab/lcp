use crate::context::CommitmentContext;
use crate::prelude::*;
use crate::{Error, StateID};
use core::fmt::Display;
use lcp_types::{Any, Height, Time};
use prost::Message;
use serde::{Deserialize, Serialize};

pub const COMMITMENT_SCHEMA_VERSION: u16 = 1;
pub const COMMITMENT_TYPE_UPDATE_CLIENT: u16 = 1;
pub const COMMITMENT_TYPE_STATE: u16 = 2;
pub const COMMITMENT_HEADER_SIZE: usize = 32;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Commitment {
    UpdateClient(UpdateClientCommitment),
    State(StateCommitment),
}

pub trait EthABIEncoder {
    fn ethabi_encode(self) -> Vec<u8>;
    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error>
    where
        Self: Sized;
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
        self.ethabi_encode()
    }

    pub fn from_commitment_bytes(bz: &[u8]) -> Result<Self, Error> {
        Self::ethabi_decode(bz)
    }

    // MSB first
    // 0-1:  version
    // 2-3:  commitment type
    // 4-31: reserved
    pub fn header(&self) -> [u8; COMMITMENT_HEADER_SIZE] {
        let mut header = [0u8; COMMITMENT_HEADER_SIZE];
        header[0..=1].copy_from_slice(&COMMITMENT_SCHEMA_VERSION.to_be_bytes());
        header[2..=3].copy_from_slice(&self.commitment_type().to_be_bytes());
        header
    }

    pub fn commitment_type(&self) -> u16 {
        match self {
            Commitment::UpdateClient(_) => COMMITMENT_TYPE_UPDATE_CLIENT,
            Commitment::State(_) => COMMITMENT_TYPE_STATE,
        }
    }
}

impl EthABIEncoder for Commitment {
    fn ethabi_encode(self) -> Vec<u8> {
        EthABIHeaderedCommitment {
            header: self.header().as_ref().try_into().unwrap(),
            commitment: match self {
                Commitment::UpdateClient(c) => c.ethabi_encode(),
                Commitment::State(c) => c.ethabi_encode(),
            },
        }
        .encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        let eth_abi_commitment = EthABIHeaderedCommitment::decode(bz)?;
        let (version, commitment_type) = {
            let header = &eth_abi_commitment.header;
            if header.len() != COMMITMENT_HEADER_SIZE {
                return Err(Error::invalid_commitment_header(format!(
                    "invalid header length: expected={COMMITMENT_HEADER_SIZE} actual={} header={:?}",
                    header.len(),
                    eth_abi_commitment.header
                )));
            }
            let mut version = [0u8; 2];
            version.copy_from_slice(&header[0..=1]);
            let mut commitment_type = [0u8; 2];
            commitment_type.copy_from_slice(&header[2..=3]);
            (
                u16::from_be_bytes(version),
                u16::from_be_bytes(commitment_type),
            )
        };
        if version != COMMITMENT_SCHEMA_VERSION {
            return Err(Error::invalid_commitment_header(format!(
                "invalid version: expected={} actual={} header={:?}",
                COMMITMENT_SCHEMA_VERSION, version, eth_abi_commitment.header
            )));
        }
        let commitment = eth_abi_commitment.commitment;
        match commitment_type {
            COMMITMENT_TYPE_UPDATE_CLIENT => {
                Ok(UpdateClientCommitment::ethabi_decode(&commitment)?.into())
            }
            COMMITMENT_TYPE_STATE => Ok(StateCommitment::ethabi_decode(&commitment)?.into()),
            _ => Err(Error::invalid_abi(format!(
                "invalid commitment type: {}",
                commitment_type
            ))),
        }
    }
}

// the struct is encoded as a tuple of 2 elements
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
    pub context: CommitmentContext,
}

impl From<UpdateClientCommitment> for Commitment {
    fn from(value: UpdateClientCommitment) -> Self {
        Self::UpdateClient(value)
    }
}

impl Display for UpdateClientCommitment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "prev_state_id={} new_state_id={} new_state_include={} prev_height={:?} new_height={:?} timestamp={} context={{{}}}",
            self.prev_state_id.map_or("".to_string(), |s| s.to_string()), self.new_state_id, self.new_state.is_some(), self.prev_height.map_or("".to_string(), |h| h.to_string()), self.new_height.to_string(), self.timestamp, self.context
        )
    }
}

// the struct is encoded as a tuple of 7 elements
pub(crate) struct EthABIUpdateClientCommitment {
    prev_state_id: ethabi::FixedBytes, // bytes32
    new_state_id: ethabi::FixedBytes,  // bytes32
    new_state: ethabi::Bytes,          // bytes
    prev_height: EthABIHeight,         // (u64, u64)
    new_height: EthABIHeight,          // (u64, u64)
    timestamp: ethabi::Uint,           // u128
    context: ethabi::Bytes,            // bytes
}

// the height is encoded as a tuple of 2 elements: (u64, u64)
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
            Token::Bytes(self.context),
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
            context: values.next().unwrap().into_bytes().unwrap(),
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
            context: value.context.ethabi_encode(),
        }
    }
}

impl TryFrom<EthABIUpdateClientCommitment> for UpdateClientCommitment {
    type Error = Error;
    fn try_from(value: EthABIUpdateClientCommitment) -> Result<Self, Self::Error> {
        Ok(Self {
            prev_state_id: bytes_to_bytes32(value.prev_state_id)?.map(StateID::from),
            new_state_id: value.new_state_id.as_slice().try_into()?,
            new_state: if value.new_state.is_empty() {
                None
            } else {
                Some(Any::try_from(value.new_state)?)
            },
            prev_height: value.prev_height.into(),
            new_height: value.new_height.into(),
            timestamp: Time::from_unix_timestamp_nanos(value.timestamp.as_u128())?,
            context: CommitmentContext::ethabi_decode(value.context.as_slice())?,
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

impl EthABIEncoder for UpdateClientCommitment {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIUpdateClientCommitment>::into(self).encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
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
}

impl EthABIEncoder for StateCommitment {
    fn ethabi_encode(self) -> Vec<u8> {
        Into::<EthABIStateCommitment>::into(self).encode()
    }

    fn ethabi_decode(bz: &[u8]) -> Result<Self, Error> {
        EthABIStateCommitment::decode(bz).and_then(|v| v.try_into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommitmentProof, TrustingPeriodContext};
    use crypto::Address;
    use lcp_types::{nanos_to_duration, MAX_UNIX_TIMESTAMP_NANOS};
    use proptest::prelude::*;
    use prost_types::Any as ProtoAny;

    fn height_from_tuple(tuple: (u64, u64)) -> Height {
        Height::new(tuple.0, tuple.1)
    }

    fn test_update_client_commitment(
        c1: UpdateClientCommitment,
        proof_signer: Address,
        proof_signature: Vec<u8>,
    ) {
        let v = c1.clone().ethabi_encode();
        let c2 = UpdateClientCommitment::ethabi_decode(&v).unwrap();
        assert_eq!(c1, c2);

        let p1 = CommitmentProof {
            commitment_bytes: Commitment::from(c1).to_commitment_bytes(),
            signer: proof_signer,
            signature: proof_signature.to_vec(),
        };
        // TODO uncomment this line when we want to generate the test data
        // println!("{{\"{}\"}},", hex::encode(p1.clone().ethabi_encode()));
        let p2 = CommitmentProof::ethabi_decode(&p1.clone().ethabi_encode()).unwrap();
        assert_eq!(p1, p2);
    }

    proptest! {
        #[test]
        fn pt_update_client_commitment_with_empty_context(
            prev_state_id in any::<Option<[u8; 32]>>().prop_map(|v| v.map(StateID::from)),
            new_state_id in any::<[u8; 32]>().prop_map(StateID::from),
            new_state in any::<Option<(String, Vec<u8>)>>().prop_filter("type_url length must be greater than 0", |t| t.is_none() || t.as_ref().unwrap().0.len() > 0),
            prev_height in any::<Option<(u64, u64)>>().prop_map(|v| v.map(height_from_tuple)),
            new_height in any::<(u64, u64)>().prop_map(height_from_tuple),
            timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS,
            proof_signer in any::<[u8; 20]>(),
            proof_signature in any::<[u8; 65]>()
        ) {
            let c1 = UpdateClientCommitment {
                prev_state_id,
                new_state_id,
                new_state: new_state.map(|(type_url, value)| {
                    ProtoAny {
                        type_url,
                        value,
                    }.try_into()
                    .unwrap()
                }),
                prev_height,
                new_height,
                timestamp: Time::from_unix_timestamp_nanos(timestamp).unwrap(),
                context: Default::default(),
            };
            test_update_client_commitment(c1, Address(proof_signer), proof_signature.to_vec());
        }

        #[test]
        fn pt_update_client_commitment_with_trusting_period_context(
            prev_state_id in any::<Option<[u8; 32]>>().prop_map(|v| v.map(StateID::from)),
            new_state_id in any::<[u8; 32]>().prop_map(StateID::from),
            new_state in any::<Option<(String, Vec<u8>)>>().prop_filter("type_url length must be greater than 0", |t| t.is_none() || t.as_ref().unwrap().0.len() > 0),
            prev_height in any::<Option<(u64, u64)>>().prop_map(|v| v.map(height_from_tuple)),
            new_height in any::<(u64, u64)>().prop_map(height_from_tuple),
            timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS,
            proof_signer in any::<[u8; 20]>(),
            proof_signature in any::<[u8; 65]>(),
            trusting_period in ..=MAX_UNIX_TIMESTAMP_NANOS,
            clock_drift in ..=MAX_UNIX_TIMESTAMP_NANOS,
            untrusted_header_timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS,
            trusted_state_timestamp in ..=MAX_UNIX_TIMESTAMP_NANOS
        ) {
            let c1 = UpdateClientCommitment {
                prev_state_id,
                new_state_id,
                new_state: new_state.map(|(type_url, value)| {
                    ProtoAny {
                        type_url,
                        value,
                    }.try_into()
                    .unwrap()
                }),
                prev_height,
                new_height,
                timestamp: Time::from_unix_timestamp_nanos(timestamp).unwrap(),
                context: TrustingPeriodContext::new(
                    nanos_to_duration(trusting_period).unwrap(),
                    nanos_to_duration(clock_drift).unwrap(),
                    Time::from_unix_timestamp_nanos(untrusted_header_timestamp).unwrap(),
                    Time::from_unix_timestamp_nanos(trusted_state_timestamp).unwrap(),
                ).into(),
            };
            test_update_client_commitment(c1, Address(proof_signer), proof_signature.to_vec());
        }

        #[test]
        fn pt_state_commitment(
            prefix in any::<CommitmentPrefix>(),
            path in any::<String>(),
            value in any::<Option<[u8; 32]>>(),
            height in any::<(u64, u64)>().prop_map(height_from_tuple),
            state_id in any::<[u8; 32]>().prop_map(StateID::from),
            proof_signer in any::<[u8; 20]>(),
            proof_signature in any::<[u8; 65]>()
        ) {
            let c1 = StateCommitment {
                prefix,
                path,
                value,
                height,
                state_id,
            };
            let v = c1.clone().ethabi_encode();
            let c2 = StateCommitment::ethabi_decode(&v).unwrap();
            assert_eq!(c1, c2);

            let p1 = CommitmentProof {
                commitment_bytes: Commitment::from(c1).to_commitment_bytes(),
                signer: Address(proof_signer),
                signature: proof_signature.to_vec(),
            };
            let p2 = CommitmentProof::ethabi_decode(&p1.clone().ethabi_encode()).unwrap();
            assert_eq!(p1, p2);
        }
    }
}
