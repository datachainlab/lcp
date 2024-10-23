use crate::relayer::Relayer;
use crate::types::to_relayer_chain_id;
use envconfig::Envconfig;
use ibc::core::ics24_host::identifier::ChainId;
use ibc_relayer::chain::ChainType;
use ibc_relayer::config::{self, ChainConfig};
use ibc_relayer::keyring::Store;
use std::str::FromStr;
use std::{sync::Arc, time::Duration};
use tendermint_rpc::Url;
use tokio::runtime::Runtime as TokioRuntime;

type Result<T> = std::result::Result<T, anyhow::Error>;

#[derive(Envconfig)]
pub struct TestChainConfig {
    #[envconfig(from = "TEST_NODE_CHAIN_ID", default = "ibc0")]
    pub id: String,
    #[envconfig(from = "TEST_NODE_RPC_ADDR", default = "http://localhost:26657")]
    pub rpc_addr: String,
    #[envconfig(
        from = "TEST_NODE_WEBSOCKET_ADDR",
        default = "ws://localhost:26657/websocket"
    )]
    pub websocket_addr: String,
    #[envconfig(from = "TEST_NODE_GRPC_ADDR", default = "http://localhost:9090")]
    pub grpc_addr: String,
}

pub fn create_relayer(rt: Arc<TokioRuntime>) -> Result<Relayer> {
    let cfg = TestChainConfig::init_from_env().unwrap();
    Relayer::new(make_tm_config(cfg), rt)
}

fn make_tm_config(cfg: TestChainConfig) -> ChainConfig {
    ChainConfig {
        id: to_relayer_chain_id(ChainId::new(cfg.id, 0)),
        r#type: ChainType::CosmosSdk,
        rpc_addr: Url::from_str(&cfg.rpc_addr).unwrap(),
        websocket_addr: Url::from_str(&cfg.websocket_addr).unwrap(),
        grpc_addr: Url::from_str(&cfg.grpc_addr).unwrap(),
        rpc_timeout: Duration::from_secs(10),
        account_prefix: "cosmos".to_string(),
        key_name: "testkey".to_string(),

        // By default we use in-memory key store to avoid polluting
        // ~/.hermes/keys. See
        // https://github.com/informalsystems/ibc-rs/issues/1541
        key_store_type: Store::Memory,

        store_prefix: "ibc".to_string(),
        default_gas: None,
        max_gas: Some(3000000),
        gas_adjustment: Some(0.1),
        gas_multiplier: None,
        fee_granter: None,
        max_msg_num: Default::default(),
        max_tx_size: Default::default(),
        max_block_time: Default::default(),
        clock_drift: Duration::from_secs(30),
        trusting_period: Some(Duration::from_secs(14 * 24 * 3600)),
        trust_threshold: Default::default(),
        gas_price: config::GasPrice::new(0.001, "stake".to_string()),
        packet_filter: Default::default(),
        address_type: Default::default(),
        memo_prefix: Default::default(),
        proof_specs: Default::default(),
        sequential_batch_tx: Default::default(),
        extension_options: Default::default(),
    }
}
