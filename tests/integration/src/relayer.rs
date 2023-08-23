use crate::types::{
    relayer_header_to_any, to_ibc_channel, to_ibc_client_state, to_ibc_consensus_state,
    to_ibc_height, to_relayer_channel_id, to_relayer_client_state, to_relayer_height,
    to_relayer_port_id,
};
use anyhow::Result;
use ibc::clients::ics07_tendermint::client_state::ClientState;
use ibc::clients::ics07_tendermint::consensus_state::ConsensusState;
use ibc::core::ics04_channel::channel::ChannelEnd;
use ibc::core::ics23_commitment::merkle::MerkleProof;
use ibc::core::ics24_host::identifier::{ChannelId, PortId};
use ibc::Height;
use ibc_relayer::chain::{
    client::ClientSettings,
    cosmos::{client::Settings, CosmosSdkChain},
    endpoint::ChainEndpoint,
    requests::{IncludeProof, QueryChannelRequest, QueryHeight},
};
use ibc_relayer::client_state::AnyClientState;
use ibc_relayer::config::ChainConfig;
use ibc_relayer::light_client::tendermint::LightClient as TmLightClient;
use ibc_relayer::light_client::{tendermint::LightClient, LightClient as IBCLightClient};
use lcp_proto::google::protobuf::Any as IBCAny;
use lcp_types::Any;
use std::sync::Arc;
use tendermint_rpc::{Client, HttpClient};
use tokio::runtime::Runtime as TokioRuntime;

pub struct Relayer {
    tmlc: LightClient,
    chain: CosmosSdkChain,

    client_state: Option<ClientState>,
}

/// Initialize the light client for the given chain using the given HTTP client
/// to fetch the node identifier to be used as peer id in the light client.
async fn init_light_client(rpc_client: &HttpClient, config: &ChainConfig) -> TmLightClient {
    use tendermint_light_client_verifier::types::PeerId;

    let peer_id: PeerId = rpc_client.status().await.map(|s| s.node_info.id).unwrap();
    TmLightClient::from_config(config, peer_id).unwrap()
}

impl Relayer {
    pub fn new(cc: ChainConfig, rt: Arc<TokioRuntime>) -> Result<Relayer> {
        let chain = CosmosSdkChain::bootstrap(cc.clone(), rt.clone()).unwrap();
        let rpc_client = HttpClient::new(cc.rpc_addr.clone()).unwrap();
        let tmlc = rt.block_on(init_light_client(&rpc_client, &cc));
        Ok(Self {
            tmlc,
            chain,
            client_state: None,
        })
    }

    pub fn create_header(&mut self, trusted_height: Height, target_height: Height) -> Result<Any> {
        let (target, supporting) = self.chain.build_header(
            to_relayer_height(trusted_height),
            to_relayer_height(target_height),
            &AnyClientState::Tendermint(to_relayer_client_state(
                self.client_state.clone().unwrap(),
            )),
        )?;
        assert!(supporting.is_empty());
        Ok(relayer_header_to_any(target))
    }

    pub fn fetch_state(&mut self, height: Height) -> Result<(ClientState, ConsensusState)> {
        let height = to_relayer_height(height);
        let block = self.tmlc.fetch(height)?;
        let config = self.chain.config();
        let client_state = to_ibc_client_state(self.chain.build_client_state(
            height,
            ClientSettings::Tendermint(Settings {
                max_clock_drift: config.clock_drift,
                trusting_period: config.trusting_period,
                trust_threshold: config.trust_threshold.into(),
            }),
        )?);
        let consensus_state = to_ibc_consensus_state(self.chain.build_consensus_state(block)?);
        self.client_state = Some(client_state.clone());
        Ok((client_state, consensus_state))
    }

    pub fn fetch_state_as_any(&mut self, height: Height) -> Result<(Any, Any)> {
        let (client_state, consensus_state) = self.fetch_state(height)?;
        let any_client_state = IBCAny::from(client_state);
        let any_consensus_state = IBCAny::from(consensus_state);
        Ok((any_client_state.into(), any_consensus_state.into()))
    }

    pub fn query_latest_height(&self) -> Result<Height> {
        Ok(to_ibc_height(self.chain.query_chain_latest_height()?))
    }

    pub fn query_channel_proof(
        &self,
        port_id: PortId,
        channel_id: ChannelId,
        height: Option<Height>, // height of consensus state
    ) -> Result<(ChannelEnd, MerkleProof, Height)> {
        let height = match height {
            Some(height) => height.decrement().unwrap(),
            None => self.query_latest_height()?.decrement().unwrap(),
        };
        let req = QueryChannelRequest {
            port_id: to_relayer_port_id(port_id),
            channel_id: to_relayer_channel_id(channel_id),
            height: QueryHeight::Specific(to_relayer_height(height)),
        };
        let res = self.chain.query_channel(req, IncludeProof::Yes)?;
        Ok((
            to_ibc_channel(res.0),
            MerkleProof {
                proofs: res.1.unwrap().proofs,
            },
            height.increment(),
        ))
    }
}
