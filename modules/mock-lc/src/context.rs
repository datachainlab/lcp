use crate::ibc::client_state::{MockClientContext, MockClientState};
use crate::ibc::consensus_state::MockConsensusState;
use crate::prelude::*;
use light_client::impl_ibc_context;

impl_ibc_context!(MockClientIBCContext, MockClientState, MockConsensusState);

impl<'a> MockClientContext for MockClientIBCContext<'a> {
    fn host_timestamp(
        &self,
    ) -> Result<ibc_primitives::Timestamp, ibc_core_host_types::error::HostError> {
        Ok(self.parent.host_timestamp().into())
    }

    fn host_height(
        &self,
    ) -> Result<ibc_core_client::types::Height, ibc_core_host_types::error::HostError> {
        // NOTE: This is a mock implementation, so we return a dummy height.
        Ok(ibc_core_client::types::Height::min(0))
    }
}
