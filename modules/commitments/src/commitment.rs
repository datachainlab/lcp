#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::CommitmentError as Error;
use crate::StateID;
use core::str::FromStr;
use ibc::core::ics23_commitment::commitment::CommitmentPrefix;
use ibc::core::ics24_host::Path;
use lcp_types::{Any, Height, Time};
use prost::Message;
use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::format;
use std::string::{String, ToString};
use std::vec::Vec;
use validation_context::ValidationParams;

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

impl Default for UpdateClientCommitment {
    fn default() -> Self {
        UpdateClientCommitment {
            prev_state_id: Default::default(),
            new_state_id: Default::default(),
            new_state: Default::default(),
            prev_height: Default::default(),
            new_height: Default::default(),
            timestamp: Time::unix_epoch(),
            validation_params: Default::default(),
        }
    }
}

impl UpdateClientCommitment {
    pub fn to_vec(&self) -> Vec<u8> {
        let mut st = rlp::RlpStream::new_list(7);
        match self.prev_state_id {
            Some(state_id) => st.append(&state_id.to_vec()),
            None => st.append_empty_data(),
        };
        st.append(&self.new_state_id.to_vec());

        match self.new_state.as_ref() {
            Some(s) => st.append(&s.encode_to_vec()),
            None => st.append_empty_data(),
        };
        match self.prev_height {
            Some(h) => st.append(&Into::<Vec<u8>>::into(h)),
            None => st.append_empty_data(),
        };
        st.append(&Into::<Vec<u8>>::into(self.new_height));
        st.append(
            &self
                .timestamp
                .as_unix_timestamp_nanos()
                .to_be_bytes()
                .as_slice(),
        );
        st.append(&self.validation_params.to_vec());
        st.out().to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let r = rlp::Rlp::new(bz);
        Ok(Self {
            prev_state_id: match r.at(0)?.as_val::<Vec<u8>>()? {
                ref v if v.len() > 0 => Some(v.as_slice().try_into()?),
                _ => None,
            },
            new_state_id: r.at(1)?.as_val::<Vec<u8>>()?.as_slice().try_into()?,
            new_state: match r.at(2)?.as_val::<Vec<u8>>()? {
                v if v.len() > 0 => Some(Any::try_from(v).map_err(Error::ICS02Error)?),
                _ => None,
            },
            prev_height: match r.at(3)?.as_val::<Vec<u8>>()?.as_slice() {
                v if v.len() > 0 => Some(v.try_into()?),
                _ => None,
            },
            new_height: r.at(4)?.as_val::<Vec<u8>>()?.as_slice().try_into()?,
            timestamp: Time::from_unix_timestamp_nanos(u128_from_bytes(
                r.at(5)?.as_val::<Vec<u8>>()?.as_slice(),
            )?)?,
            validation_params: ValidationParams::from_bytes(
                r.at(6)?.as_val::<Vec<u8>>()?.as_slice(),
            ),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StateCommitment {
    pub prefix: CommitmentPrefix,
    pub path: Path,
    pub value: Option<[u8; 32]>,
    pub height: Height,
    pub state_id: StateID,
}

impl StateCommitment {
    pub fn new(
        prefix: CommitmentPrefix,
        path: Path,
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
        let mut st = rlp::RlpStream::new_list(5);
        st.append(&self.prefix.as_bytes());
        st.append(&self.path.to_string());
        if let Some(value) = self.value {
            st.append(&value.as_slice());
        } else {
            st.append_empty_data();
        }
        st.append(&Into::<Vec<u8>>::into(self.height));
        st.append(&self.state_id.to_vec());
        st.out().to_vec()
    }

    pub fn from_bytes(bz: &[u8]) -> Result<Self, Error> {
        let r = rlp::Rlp::new(bz);
        Ok(Self {
            prefix: r
                .at(0)?
                .as_val::<Vec<u8>>()?
                .try_into()
                .map_err(Error::ICS23Error)?,
            path: Path::from_str(&r.at(1)?.as_val::<String>()?).map_err(Error::ICS24PathError)?,
            value: match r.at(2)?.as_val::<Vec<u8>>()?.as_slice() {
                bz if bz.len() > 0 => Some(bytes_to_array(bz)?),
                _ => None,
            },
            height: r.at(3)?.as_val::<Vec<u8>>()?.as_slice().try_into()?,
            state_id: r.at(4)?.as_val::<Vec<u8>>()?.as_slice().try_into()?,
        })
    }
}

fn u128_from_bytes(bz: &[u8]) -> Result<u128, Error> {
    if bz.len() == 16 {
        let mut ar: [u8; 16] = Default::default();
        ar.copy_from_slice(bz);
        Ok(u128::from_be_bytes(ar))
    } else {
        Err(Error::InvalidCommitmentFormatError(format!(
            "bytes length must be 16, but got {}",
            bz.len()
        )))
    }
}

fn bytes_to_array(bz: &[u8]) -> Result<[u8; 32], Error> {
    if bz.len() == 32 {
        let mut ar: [u8; 32] = Default::default();
        ar.copy_from_slice(bz);
        Ok(ar)
    } else {
        Err(Error::InvalidCommitmentFormatError(format!(
            "bytes length must be 32, but got {}",
            bz.len()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ibc::core::{ics02_client::client_type::ClientType, ics24_host::identifier::ClientId};
    use prost_types::Any as ProtoAny;
    use rand::{distributions::Uniform, thread_rng, Rng};

    #[test]
    fn test_update_client_commitment_converter() {
        for _ in 0..256 {
            let c1 = UpdateClientCommitment {
                prev_state_id: rand_or_none(gen_rand_state_id),
                new_state_id: gen_rand_state_id(),
                new_state: rand_or_none(|| -> Any {
                    ProtoAny {
                        type_url: "/".to_owned(),
                        value: gen_rand_vec(64),
                    }
                    .into()
                }),
                prev_height: rand_or_none(gen_rand_height),
                new_height: gen_rand_height(),
                timestamp: Time::now(),
                validation_params: Default::default(),
            };
            let v = c1.to_vec();
            let c2 = UpdateClientCommitment::from_bytes(&v).unwrap();
            assert_eq!(c1, c2);
        }
    }

    #[test]
    fn test_state_commitment_converter() {
        for _ in 0..256 {
            let c1 = StateCommitment {
                prefix: "ibc".as_bytes().to_vec().try_into().unwrap(),
                path: Path::ClientType(ibc::core::ics24_host::path::ClientTypePath(
                    ClientId::new(ClientType::Tendermint, thread_rng().gen()).unwrap(),
                )),
                value: rand_or_none(|| bytes_to_array(gen_rand_vec(32).as_slice()).unwrap()),
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
        let vals: Vec<u8> = (0..size).map(|_| rng.sample(&range)).collect();
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
