use anyhow::Result;
use ibc::clients::ics07_tendermint::client_state::ClientState;
use ibc::clients::ics07_tendermint::consensus_state::ConsensusState;
use ibc::core::ics02_client::header::AnyHeader;
use ibc::core::ics02_client::{client_consensus::AnyConsensusState, client_state::AnyClientState};
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics24_host::identifier::{ChannelId, PortId};
use ibc::Height;
use ibc_proto::ibc::core::commitment::v1::MerkleProof;
use ibc_relayer::chain::{ChainEndpoint, CosmosSdkChain};
use ibc_relayer::config::ChainConfig;
use ibc_relayer::light_client::tendermint::LightClient;
use ibc_relayer::light_client::LightClient as IBCLightClient;
use prost_types::Any;
use std::sync::Arc;
use tokio::runtime::Runtime as TokioRuntime;

pub struct Relayer {
    tmlc: LightClient,
    chain: CosmosSdkChain,

    client_state: Option<AnyClientState>,
}

impl Relayer {
    pub fn new(cc: ChainConfig, rt: Arc<TokioRuntime>) -> Result<Relayer> {
        let chain = CosmosSdkChain::bootstrap(cc, rt).unwrap();
        let tmlc = chain.init_light_client()?;
        Ok(Self {
            tmlc,
            chain,
            client_state: None,
        })
    }

    pub fn create_header(
        &mut self,
        trusted_height: Height,
        target_height: Height,
    ) -> Result<AnyHeader> {
        let (target, supporting) = self.chain.build_header(
            trusted_height,
            target_height,
            self.client_state.as_ref().unwrap(),
            &mut self.tmlc,
        )?;
        assert!(supporting.len() == 0);
        Ok(ibc::core::ics02_client::header::AnyHeader::Tendermint(
            target,
        ))
    }

    pub fn fetch_state(&mut self, height: Height) -> Result<(ClientState, ConsensusState)> {
        let block = self.tmlc.fetch(height)?;
        let client_state = self
            .chain
            .build_client_state(height, self.chain.config().to_owned())?;
        let consensus_state = self.chain.build_consensus_state(block)?;

        self.client_state = Some(AnyClientState::Tendermint(client_state.clone()));

        Ok((client_state, consensus_state))
    }

    pub fn fetch_state_as_any(&mut self, height: Height) -> Result<(Any, Any)> {
        let (client_state, consensus_state) = self.fetch_state(height)?;
        Ok((
            Any::from(AnyClientState::Tendermint(client_state)),
            Any::from(AnyConsensusState::Tendermint(consensus_state)),
        ))
    }

    pub fn query_latest_height(&self) -> Result<Height> {
        let height = self.chain.query_latest_height()?;
        Ok(height)
    }

    pub fn proven_channel(
        &self,
        port_id: &PortId,
        channel_id: &ChannelId,
        height: Option<Height>,
    ) -> Result<(ChannelEnd, MerkleProof, Height)> {
        let height = match height {
            Some(height) => height,
            None => self.query_latest_height()?,
        };
        let res = self.chain.proven_channel(port_id, channel_id, height)?;
        Ok((res.0, res.1, height))
    }
}
