use crate::errors::InputValidationError as Error;
use crate::light_client::*;
use crate::prelude::*;
use core::str::FromStr;
use crypto::Address;
use lcp_types::proto::lcp::service::elc::v1::{
    MsgAggregateMessages, MsgAggregateMessagesResponse, MsgCreateClient, MsgCreateClientResponse,
    MsgUpdateClient, MsgUpdateClientResponse, MsgVerifyMembership, MsgVerifyMembershipResponse,
    MsgVerifyNonMembership, MsgVerifyNonMembershipResponse, QueryClientRequest,
    QueryClientResponse,
};
use lcp_types::{ClientId, Time};

impl TryFrom<MsgCreateClient> for InitClientInput {
    type Error = Error;
    fn try_from(msg: MsgCreateClient) -> Result<Self, Error> {
        let any_client_state = msg
            .client_state
            .ok_or_else(|| Error::invalid_argument("client_state must be non-nil".into()))?
            .into();
        let any_consensus_state = msg
            .consensus_state
            .ok_or_else(|| Error::invalid_argument("consensus_state must be non-nil".into()))?
            .into();
        Ok(Self {
            any_client_state,
            any_consensus_state,
            current_timestamp: Time::now(),
            signer: Address::try_from(msg.signer.as_slice())?,
        })
    }
}

impl TryFrom<MsgUpdateClient> for UpdateClientInput {
    type Error = Error;
    fn try_from(msg: MsgUpdateClient) -> Result<Self, Error> {
        let any_header = msg
            .header
            .ok_or_else(|| Error::invalid_argument("header must be non-nil".into()))?
            .into();
        let client_id = ClientId::from_str(&msg.client_id)?;
        Ok(Self {
            client_id,
            any_header,
            include_state: msg.include_state,
            current_timestamp: Time::now(),
            signer: Address::try_from(msg.signer.as_slice())?,
        })
    }
}

impl TryFrom<MsgAggregateMessages> for AggregateMessagesInput {
    type Error = Error;
    fn try_from(msg: MsgAggregateMessages) -> Result<Self, Error> {
        let signer = Address::try_from(msg.signer.as_slice())?;
        Ok(Self {
            signer,
            messages: msg.messages,
            signatures: msg.signatures,
            current_timestamp: Time::now(),
        })
    }
}

impl TryFrom<MsgVerifyMembership> for VerifyMembershipInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyMembership) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::invalid_argument("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        Ok(Self {
            client_id,
            prefix: msg.prefix,
            proof,
            path: msg.path,
            value: msg.value,
            signer: Address::try_from(msg.signer.as_slice())?,
        })
    }
}

impl TryFrom<MsgVerifyNonMembership> for VerifyNonMembershipInput {
    type Error = Error;

    fn try_from(msg: MsgVerifyNonMembership) -> Result<Self, Self::Error> {
        let client_id = ClientId::from_str(&msg.client_id)?;
        let proof = CommitmentProofPair(
            msg.proof_height
                .ok_or_else(|| Error::invalid_argument("proof_height must be non-nil".into()))?
                .into(),
            msg.proof,
        );
        Ok(Self {
            client_id,
            prefix: msg.prefix,
            proof,
            path: msg.path,
            signer: Address::try_from(msg.signer.as_slice())?,
        })
    }
}

impl TryFrom<QueryClientRequest> for QueryClientInput {
    type Error = Error;
    fn try_from(query: QueryClientRequest) -> Result<Self, Error> {
        let client_id = ClientId::from_str(&query.client_id)?;
        Ok(Self { client_id })
    }
}

impl From<InitClientResult> for MsgCreateClientResponse {
    fn from(res: InitClientResult) -> Self {
        Self {
            client_id: res.client_id.to_string(),
            message: res.proof.message,
            signer: res.proof.signer.into(),
            signature: res.proof.signature,
        }
    }
}

impl From<UpdateClientResult> for MsgUpdateClientResponse {
    fn from(res: UpdateClientResult) -> Self {
        Self {
            message: res.0.message,
            signer: res.0.signer.into(),
            signature: res.0.signature,
        }
    }
}

impl From<AggregateMessagesResult> for MsgAggregateMessagesResponse {
    fn from(res: AggregateMessagesResult) -> Self {
        Self {
            message: res.0.message,
            signer: res.0.signer.into(),
            signature: res.0.signature,
        }
    }
}

impl From<VerifyMembershipResult> for MsgVerifyMembershipResponse {
    fn from(res: VerifyMembershipResult) -> Self {
        Self {
            message: res.0.message,
            signer: res.0.signer.to_vec(),
            signature: res.0.signature,
        }
    }
}

impl From<VerifyNonMembershipResult> for MsgVerifyNonMembershipResponse {
    fn from(res: VerifyNonMembershipResult) -> Self {
        Self {
            message: res.0.message,
            signer: res.0.signer.to_vec(),
            signature: res.0.signature,
        }
    }
}

impl From<QueryClientResult> for QueryClientResponse {
    fn from(res: QueryClientResult) -> Self {
        Self {
            client_state: Some(res.any_client_state.into()),
            consensus_state: Some(res.any_consensus_state.into()),
        }
    }
}
