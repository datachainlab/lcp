#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::{UpdateClientCommitment, UpdateClientCommitmentProof};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::header::AnyHeader;
use ibc::timestamp::Timestamp;
use ibc::Height;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Header(
    pub UpdateClientCommitmentProof,
    #[serde(skip_serializing)]pub UpdateClientCommitment,
);

impl Header {
    pub fn commitment(&self) -> &UpdateClientCommitment {
        &self.1
    }
}

impl ibc::core::ics02_client::header::Header for Header {
    fn client_type(&self) -> ClientType {
        // NOTE: ClientType is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }

    fn height(&self) -> Height {
        self.commitment().new_height.clone()
    }

    fn timestamp(&self) -> Timestamp {
        Timestamp::from_nanoseconds(self.commitment().timestamp).unwrap()
    }

    fn wrap_any(self) -> AnyHeader {
        // NOTE: AnyHeader is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }
}
