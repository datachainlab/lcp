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
    fn maybe_consensus_state(
        ctx: &dyn ValidationContext,
        client_cons_state_path: &ClientConsensusStatePath,
    ) -> Result<Option<Box<dyn ConsensusState>>, ClientError> {
        match ctx.consensus_state(client_cons_state_path) {
            Ok(cs) => Ok(Some(cs)),
            Err(e) => match e {
                ContextError::ClientError(ClientError::ConsensusStateNotFound {
                    client_id: _,
                    height: _,
                }) => Ok(None),
                ContextError::ClientError(e) => Err(e),
                _ => Err(ClientError::Other {
                    description: e.to_string(),
                }),
            },
        }
    }

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

    // Check if a consensus state is already installed; if so it should
    // match the untrusted header.
    let header_consensus_state = TmConsensusState::from(header.clone());
    let client_cons_state_path = ClientConsensusStatePath::new(&client_id, &header.height());
    let existing_consensus_state = match maybe_consensus_state(ctx, &client_cons_state_path)? {
        Some(cs) => {
            let cs = downcast_tm_consensus_state(cs.as_ref())?;
            // If this consensus state matches, skip verification
            // (optimization)
            if cs == header_consensus_state {
                // Header is already installed and matches the incoming
                // header (already verified)
                return Ok(UpdatedState {
                    client_state: client_state.into_box(),
                    consensus_state: cs.into_box(),
                });
            }
            Some(cs)
        }
        None => None,
    };

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

    // If the header has verified, but its corresponding consensus state
    // differs from the existing consensus state for that height, freeze the
    // client and return the installed consensus state.
    if let Some(cs) = existing_consensus_state {
        if cs != header_consensus_state {
            return Ok(UpdatedState {
                client_state: client_state.with_frozen_height(header.height()).into_box(),
                consensus_state: cs.into_box(),
            });
        }
    }

    // Monotonicity checks for timestamps for in-the-middle updates
    // (cs-new, cs-next, cs-latest)
    if header.height() < client_state.latest_height() {
        let maybe_next_cs = ctx
            .next_consensus_state(&client_id, &header.height())
            .map_err(|e| match e {
                ContextError::ClientError(e) => e,
                _ => ClientError::Other {
                    description: e.to_string(),
                },
            })?
            .as_ref()
            .map(|cs| downcast_tm_consensus_state(cs.as_ref()))
            .transpose()?;

        if let Some(next_cs) = maybe_next_cs {
            // New (untrusted) header timestamp cannot occur after next
            // consensus state's height
            if header.signed_header.header().time > next_cs.timestamp {
                return Err(ClientError::ClientSpecific {
                    description: Error::HeaderTimestampTooHigh {
                        actual: header.signed_header.header().time.to_string(),
                        max: next_cs.timestamp.to_string(),
                    }
                    .to_string(),
                });
            }
        }
    }

    // (cs-trusted, cs-prev, cs-new)
    if header.trusted_height < header.height() {
        let maybe_prev_cs = ctx
            .prev_consensus_state(&client_id, &header.height())
            .map_err(|e| match e {
                ContextError::ClientError(e) => e,
                _ => ClientError::Other {
                    description: e.to_string(),
                },
            })?
            .as_ref()
            .map(|cs| downcast_tm_consensus_state(cs.as_ref()))
            .transpose()?;

        if let Some(prev_cs) = maybe_prev_cs {
            // New (untrusted) header timestamp cannot occur before the
            // previous consensus state's height
            if header.signed_header.header().time < prev_cs.timestamp {
                return Err(ClientError::ClientSpecific {
                    description: Error::HeaderTimestampTooLow {
                        actual: header.signed_header.header().time.to_string(),
                        min: prev_cs.timestamp.to_string(),
                    }
                    .to_string(),
                });
            }
        }
    }

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
        .map(Clone::clone)
}
