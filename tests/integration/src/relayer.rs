use ibc::core::ics24_host::identifier::ChainId;
use ibc_relayer::config::{self, ChainConfig};
use ibc_relayer::keyring::Store;
use relay_tendermint::Relayer;
use std::str::FromStr;
use std::{sync::Arc, time::Duration};
use tendermint_rpc::Url;
use tokio::runtime::Runtime as TokioRuntime;

type Result<T> = std::result::Result<T, anyhow::Error>;

pub fn create_relayer(rt: Arc<TokioRuntime>, chain_id: &str) -> Result<Relayer> {
    Relayer::new(make_tm_config(chain_id), rt)
}

fn make_tm_config(chain_id: &str) -> ChainConfig {
    ChainConfig {
        id: ChainId::new(chain_id.into(), 0),
        rpc_addr: Url::from_str("http://localhost:26657").unwrap(),
        websocket_addr: Url::from_str("ws://localhost:26657/websocket").unwrap(),
        grpc_addr: Url::from_str("http://localhost:9090").unwrap(),
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
        fee_granter: None,
        max_msg_num: Default::default(),
        max_tx_size: Default::default(),
        max_block_time: Default::default(),
        clock_drift: Duration::from_secs(5),
        trusting_period: Some(Duration::from_secs(14 * 24 * 3600)),
        trust_threshold: Default::default(),
        gas_price: config::GasPrice::new(0.001, "stake".to_string()),
        packet_filter: Default::default(),
        address_type: Default::default(),
        memo_prefix: Default::default(),
        proof_specs: Default::default(),
    }
}
