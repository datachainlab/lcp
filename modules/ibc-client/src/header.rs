#[cfg(feature = "sgx")]
use crate::sgx_reexport_prelude::*;
use crate::{crypto::Address, report::AttestationVerificationReport};
use attestation_report::EndorsedAttestationReport;
use commitments::{StateID, UpdateClientCommitment, UpdateClientCommitmentProof};
use ibc::core::ics02_client::client_type::ClientType;
use ibc::core::ics02_client::header::AnyHeader;
use ibc::timestamp::Timestamp;
use ibc::Height;
use serde::{Deserialize, Serialize};
use validation_context::ValidationParams;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Header {
    Activate(ActivateHeader),
    RegisterEnclaveKey(RegisterEnclaveKeyHeader),
    UpdateClient(UpdateClientHeader),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ActivateHeader(
    pub Vec<u8>, // initial state bytes
    pub UpdateClientCommitmentProof,
    #[serde(skip_serializing)] pub UpdateClientCommitment,
);

impl Commitment for ActivateHeader {
    fn commitment_proof(&self) -> &UpdateClientCommitmentProof {
        &self.1
    }

    fn commitment(&self) -> &UpdateClientCommitment {
        &self.2
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RegisterEnclaveKeyHeader(pub AttestationVerificationReport);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct UpdateClientHeader(
    pub UpdateClientCommitmentProof,
    #[serde(skip_serializing)] pub UpdateClientCommitment,
);

impl Commitment for UpdateClientHeader {
    fn commitment_proof(&self) -> &UpdateClientCommitmentProof {
        &self.0
    }

    fn commitment(&self) -> &UpdateClientCommitment {
        &self.1
    }
}

pub trait Commitment {
    fn commitment_proof(&self) -> &UpdateClientCommitmentProof;

    fn commitment(&self) -> &UpdateClientCommitment;

    fn signer(&self) -> Address {
        self.commitment_proof().signer.as_slice().into()
    }

    fn height(&self) -> Height {
        self.commitment().new_height
    }

    fn prev_height(&self) -> Option<Height> {
        self.commitment().prev_height
    }

    fn prev_state_id(&self) -> Option<StateID> {
        self.commitment().prev_state_id
    }

    fn state_id(&self) -> StateID {
        self.commitment().new_state_id
    }

    fn timestamp_as_u128(&self) -> u128 {
        self.commitment().timestamp
    }

    fn validation_params(&self) -> &ValidationParams {
        &self.commitment().validation_params
    }
}

impl ibc::core::ics02_client::header::Header for Header {
    fn client_type(&self) -> ClientType {
        // NOTE: ClientType is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }

    fn height(&self) -> Height {
        match self {
            Header::UpdateClient(h) => h.height(),
            _ => todo!(),
        }
    }

    fn timestamp(&self) -> Timestamp {
        match self {
            Header::UpdateClient(h) => {
                Timestamp::from_nanoseconds(h.timestamp_as_u128() as u64).unwrap()
            }
            _ => todo!(),
        }
    }

    fn wrap_any(self) -> AnyHeader {
        // NOTE: AnyHeader is defined as enum in ibc-rs, so we cannot support an additional type
        todo!()
    }
}
