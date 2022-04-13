#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use commitments::UpdateClientCommitmentProof;
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::header::AnyHeader;
use ibc::timestamp::Timestamp;
use ibc::Height;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Header {
    pub commitment_bytes: Vec<u8>,
    pub signer: Vec<u8>,
    pub signature: Vec<u8>,
}

impl ibc::core::ics02_client::header::Header for Header {
    fn client_type(&self) -> ClientType {
        // NOTE: ClientType is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }

    fn height(&self) -> Height {
        todo!()
    }

    fn timestamp(&self) -> Timestamp {
        todo!()
    }

    fn wrap_any(self) -> AnyHeader {
        todo!()
    }
}
