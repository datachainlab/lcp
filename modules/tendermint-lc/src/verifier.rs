use crate::prelude::*;
use ibc::{
    clients::ics07_tendermint::{
        client_state::ClientState, client_type,
        consensus_state::ConsensusState as TmConsensusState, error::Error,
        header::Header as TmHeader,
    },
    core::{
        ics02_client::{
            client_state::{ClientState as Ics2ClientState, UpdatedState},
            consensus_state::ConsensusState,
            error::ClientError,
        },
        ics24_host::{identifier::ClientId, path::ClientConsensusStatePath},
        ContextError, ValidationContext,
    },
};
use lcp_proto::google::protobuf::Any;
use tendermint_light_client_verifier::{
    types::{TrustedBlockState, UntrustedBlockState},
    ProdVerifier, Verdict, Verifier,
};

/// Fork of the `check_header_and_update_state` function from ibc-rs v0.29.0
/// https://github.com/cosmos/ibc-rs/blob/10b47c077065a07ded9ac7f03fdb6c0980592d81/crates/ibc/src/clients/ics07_tendermint/client_state.rs#L457
pub(crate) fn check_header_and_update_state(
    client_state: &ClientState,
    ctx: &dyn ValidationContext,
    client_id: ClientId,
    header: Any,
) -> Result<UpdatedState, ClientError> {
    let client_state = downcast_tm_client_state(client_state)?.clone();
    let header = TmHeader::try_from(header)?;

    if header.height().revision_number() != client_state.chain_id().version() {
        return Err(ClientError::ClientSpecific {
            description: Error::MismatchedRevisions {
                current_revision: client_state.chain_id().version(),
                update_revision: header.height().revision_number(),
            }
            .to_string(),
        });
    }

    let trusted_client_cons_state_path =
        ClientConsensusStatePath::new(&client_id, &header.trusted_height);
    let trusted_consensus_state = downcast_tm_consensus_state(
        ctx.consensus_state(&trusted_client_cons_state_path)
            .map_err(|e| match e {
                ContextError::ClientError(e) => e,
                _ => ClientError::Other {
                    description: e.to_string(),
                },
            })?
            .as_ref(),
    )?;

    let trusted_state = TrustedBlockState {
        chain_id: &client_state.chain_id.clone().into(),
        header_time: trusted_consensus_state.timestamp,
        height: header
            .trusted_height
            .revision_height()
            .try_into()
            .map_err(|_| ClientError::ClientSpecific {
                description: Error::InvalidHeaderHeight {
                    height: header.trusted_height.revision_height(),
                }
                .to_string(),
            })?,
        next_validators: &header.trusted_validator_set,
        next_validators_hash: trusted_consensus_state.next_validators_hash,
    };

    let untrusted_state = UntrustedBlockState {
        signed_header: &header.signed_header,
        validators: &header.validator_set,
        // NB: This will skip the
        // VerificationPredicates::next_validators_match check for the
        // untrusted state.
        next_validators: None,
    };

    let options = client_state.as_light_client_options()?;
    let now = ctx
        .host_timestamp()
        .map_err(|e| ClientError::Other {
            description: e.to_string(),
        })?
        .into_tm_time()
        .unwrap();

    match ProdVerifier::default().verify(untrusted_state, trusted_state, &options, now) {
        Verdict::Success => Ok(()),
        Verdict::NotEnoughTrust(reason) => Err(Error::NotEnoughTrustedValsSigned { reason }),
        Verdict::Invalid(detail) => Err(Error::VerificationError { detail }),
    }?;

    Ok(UpdatedState {
        client_state: client_state.with_header(header.clone())?.into_box(),
        consensus_state: TmConsensusState::from(header).into_box(),
    })
}

fn downcast_tm_client_state(cs: &dyn Ics2ClientState) -> Result<&ClientState, ClientError> {
    cs.as_any()
        .downcast_ref::<ClientState>()
        .ok_or_else(|| ClientError::ClientArgsTypeMismatch {
            client_type: client_type(),
        })
}

fn downcast_tm_consensus_state(cs: &dyn ConsensusState) -> Result<TmConsensusState, ClientError> {
    cs.as_any()
        .downcast_ref::<TmConsensusState>()
        .ok_or_else(|| ClientError::ClientArgsTypeMismatch {
            client_type: client_type(),
        })
        .cloned()
}
