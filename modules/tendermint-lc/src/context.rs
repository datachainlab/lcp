use crate::prelude::*;
use ibc_client_tendermint::client_state::ClientState as TendermintClientState;
use ibc_client_tendermint::consensus_state::ConsensusState as TendermintConsensusState;
use light_client::impl_ibc_context_ext;

impl_ibc_context_ext!(
    TendermintIBCContext,
    TendermintClientState,
    TendermintConsensusState
);
