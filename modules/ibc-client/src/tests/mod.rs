#[cfg(test)]
mod tests {
    use super::*;
    use crypto::EnclaveKey;
    use enclave_commands::{Command, InitClientInput, LightClientCommand};
    use handler::router;
    use ibc::{
        clients::ics07_tendermint::{client_state::ClientState, consensus_state::ConsensusState},
        core::ics02_client::{
            client_consensus::AnyConsensusState, client_state::AnyClientState,
            client_type::ClientType,
        },
    };
    use lazy_static::lazy_static;
    use light_client::{LightClient, LightClientRegistry, LightClientSource};
    use prost_types::Any;
    use store::memory::MemStore;

    lazy_static! {
        pub static ref LIGHT_CLIENT_REGISTRY: LightClientRegistry = {
            let mut registry = LightClientRegistry::new();
            tendermint_lc::register_implementations(&mut registry);
            mock_lc::register_implementations(&mut registry);
            registry
        };
    }

    #[derive(Default)]
    struct LocalLightClientRegistry;

    impl LightClientSource<'static> for LocalLightClientRegistry {
        fn get_light_client(type_url: &str) -> Option<&'static Box<dyn LightClient>> {
            LIGHT_CLIENT_REGISTRY.get(type_url)
        }
    }

    #[test]
    fn test_example() {
        let ek = EnclaveKey::new().unwrap();
        let mut store = MemStore::new();

        // WIP setup an initial state

        let client_state = ClientState {
            chain_id: todo!(),
            trust_level: todo!(),
            trusting_period: todo!(),
            unbonding_period: todo!(),
            max_clock_drift: todo!(),
            latest_height: todo!(),
            proof_specs: todo!(),
            upgrade_path: todo!(),
            allow_update: todo!(),
            frozen_height: todo!(),
        };
        let consensus_state = ConsensusState {
            timestamp: todo!(),
            root: todo!(),
            next_validators_hash: todo!(),
        };

        let input = InitClientInput {
            client_type: ClientType::Tendermint.as_str().to_string(),
            any_client_state: Any::from(AnyClientState::Tendermint(client_state)),
            any_consensus_state: Any::from(AnyConsensusState::Tendermint(consensus_state)),
            current_timestamp: 0,
        };
        let res = router::dispatch::<_, LocalLightClientRegistry>(
            Some(&ek),
            store,
            Command::LightClient(LightClientCommand::InitClient(input)),
        );
        assert!(res.is_ok());
    }
}
