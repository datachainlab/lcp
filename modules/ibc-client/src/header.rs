use crate::crypto::Address;
#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::{StateID, UpdateClientCommitment, UpdateClientCommitmentProof};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::header::AnyHeader;
use ibc::timestamp::Timestamp;
use ibc::Height;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Header(
    pub UpdateClientCommitmentProof,
    #[serde(skip_serializing)] pub UpdateClientCommitment,
);

impl Header {
    pub fn commitment(&self) -> &UpdateClientCommitment {
        &self.1
    }

    pub fn prev_height(&self) -> Option<Height> {
        self.commitment().prev_height
    }

    pub fn prev_state_id(&self) -> Option<StateID> {
        self.commitment().prev_state_id
    }

    pub fn state_id(&self) -> StateID {
        self.commitment().new_state_id
    }

    pub fn signer(&self) -> Address {
        self.0.signer.as_slice().into()
    }

    pub fn timestamp_as_u64(&self) -> u64 {
        self.commitment().timestamp
    }
}

impl ibc::core::ics02_client::header::Header for Header {
    fn client_type(&self) -> ClientType {
        // NOTE: ClientType is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }

    fn height(&self) -> Height {
        self.commitment().new_height
    }

    fn timestamp(&self) -> Timestamp {
        Timestamp::from_nanoseconds(self.commitment().timestamp).unwrap()
    }

    fn wrap_any(self) -> AnyHeader {
        // NOTE: AnyHeader is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }
}
