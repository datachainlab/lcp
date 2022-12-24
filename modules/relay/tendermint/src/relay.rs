use anyhow::Result;
use ibc::clients::ics07_tendermint::client_state::ClientState;
use ibc::clients::ics07_tendermint::consensus_state::ConsensusState;
use ibc::core::ics02_client::header::AnyHeader;
use ibc::core::ics02_client::{client_consensus::AnyConsensusState, client_state::AnyClientState};
use ibc::core::ics03_connection::connection::ConnectionEnd;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics04_channel::commitment::PacketCommitment;
use ibc::core::ics04_channel::packet::Sequence;
use ibc::core::ics23_commitment::merkle::MerkleProof;
use ibc::core::ics24_host::identifier::{ChannelId, ConnectionId, PortId};
use ibc::Height;
use ibc_relayer::chain::requests::QueryConnectionRequest;
use ibc_relayer::chain::{
    client::ClientSettings,
    cosmos::{client::Settings, CosmosSdkChain},
    endpoint::ChainEndpoint,
    requests::{IncludeProof, QueryChannelRequest, QueryHeight, QueryPacketCommitmentRequest},
};
use ibc_relayer::config::ChainConfig;
use ibc_relayer::light_client::{tendermint::LightClient, LightClient as IBCLightClient};
use lcp_types::Any;
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
        let config = self.chain.config();
        let client_state = self.chain.build_client_state(
            height,
            ClientSettings::Tendermint(Settings {
                max_clock_drift: config.clock_drift,
                trusting_period: config.trusting_period,
                trust_threshold: config.trust_threshold.into(),
            }),
        )?;
        let consensus_state = self.chain.build_consensus_state(block)?;

        self.client_state = Some(AnyClientState::Tendermint(client_state.clone()));

        Ok((client_state, consensus_state))
    }

    pub fn fetch_state_as_any(&mut self, height: Height) -> Result<(Any, Any)> {
        let (client_state, consensus_state) = self.fetch_state(height)?;
        let client_state = AnyClientState::Tendermint(client_state);
        let consensus_state = AnyConsensusState::Tendermint(consensus_state);
        Ok((client_state.into(), consensus_state.into()))
    }

    pub fn query_latest_height(&self) -> Result<Height> {
        Ok(self.chain.query_chain_latest_height()?)
    }

    pub fn query_connection_proof(
        &self,
        connection_id: ConnectionId,
        height: Option<Height>, // height of consensus state
    ) -> Result<(ConnectionEnd, MerkleProof, Height)> {
        let height = match height {
            Some(height) => height.decrement()?,
            None => self.query_latest_height()?.decrement()?,
        };
        let req = QueryConnectionRequest {
            connection_id,
            height: QueryHeight::Specific(height),
        };
        let res = self.chain.query_connection(req, IncludeProof::Yes)?;
        Ok((res.0, res.1.unwrap(), height.increment()))
    }

    pub fn query_channel_proof(
        &self,
        port_id: PortId,
        channel_id: ChannelId,
        height: Option<Height>, // height of consensus state
    ) -> Result<(ChannelEnd, MerkleProof, Height)> {
        let height = match height {
            Some(height) => height.decrement()?,
            None => self.query_latest_height()?.decrement()?,
        };
        let req = QueryChannelRequest {
            port_id,
            channel_id,
            height: QueryHeight::Specific(height),
        };
        let res = self.chain.query_channel(req, IncludeProof::Yes)?;
        Ok((res.0, res.1.unwrap(), height.increment()))
    }

    pub fn query_packet_proof(
        &self,
        port_id: PortId,
        channel_id: ChannelId,
        sequence: Sequence,
        height: Option<Height>, // height of consensus state
    ) -> Result<(PacketCommitment, MerkleProof, Height)> {
        let height = match height {
            Some(height) => height.decrement()?,
            None => self.query_latest_height()?.decrement()?,
        };
        let res = self.chain.query_packet_commitment(
            QueryPacketCommitmentRequest {
                port_id,
                channel_id,
                sequence,
                height: QueryHeight::Specific(height),
            },
            IncludeProof::Yes,
        )?;
        Ok((res.0.into(), res.1.unwrap(), height.increment()))
    }
}
