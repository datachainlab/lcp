mod client;
mod errors;
#[cfg(test)]
mod tests {
    use super::client::register_implementations;
    use super::client::LCP_CLIENT_TYPE;
    use super::*;
    use crate::{
        client_def::LCPClient, client_state::ClientState, consensus_state::ConsensusState,
        crypto::Address, header::Header,
    };
    use crypto::EnclaveKey;
    use enclave_commands::{
        Command, CommandResult, InitClientInput, InitClientResult, LightClientCommand,
        LightClientResult, UpdateClientInput, UpdateClientResult,
    };
    use handler::router;
    use ibc::{
        core::ics02_client::{
            client_consensus::AnyConsensusState, client_state::AnyClientState,
            client_type::ClientType, header::AnyHeader,
        },
        mock::{
            client_state::{MockClientState, MockConsensusState},
            header::MockHeader,
        },
        Height,
    };
    use lazy_static::lazy_static;
    use light_client::{LightClient, LightClientRegistry, LightClientSource};
    use prost_types::Any;
    use store::memory::MemStore;

    lazy_static! {
        pub static ref LIGHT_CLIENT_REGISTRY: LightClientRegistry = {
            let mut registry = LightClientRegistry::new();
            mock_lc::register_implementations(&mut registry);
            register_implementations(&mut registry);
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
    fn test_init_client() {
        let ek = EnclaveKey::new().unwrap();
        let mut lcp_store = MemStore::new();
        let mut ibc_store = MemStore::new();

        let client = LCPClient::default();

        let proof0 = {
            let header = MockHeader::new(Height::new(0, 1));
            let client_state = MockClientState::new(header);
            let consensus_state = MockConsensusState::new(header);

            let input = InitClientInput {
                client_type: ClientType::Mock.as_str().to_string(),
                any_client_state: Any::from(AnyClientState::Mock(client_state)),
                any_consensus_state: Any::from(AnyConsensusState::Mock(consensus_state)),
                current_timestamp: 0,
            };
            assert_eq!(lcp_store.revision, 1);
            let res = router::dispatch::<_, LocalLightClientRegistry>(
                Some(&ek),
                &mut lcp_store,
                Command::LightClient(LightClientCommand::InitClient(input)),
            );
            assert!(res.is_ok(), "res={:?}", res);
            assert_eq!(lcp_store.revision, 2);
            if let CommandResult::LightClient(LightClientResult::InitClient(InitClientResult(
                proof,
            ))) = res.unwrap()
            {
                proof
            } else {
                unreachable!()
            }
        };

        {
            let initial_client_state = ClientState {
                latest_height: Height::new(0, 1),
                mr_enclave: Default::default(),
                key_expiration: 1000000,
                keys: vec![(Address::from(ek.get_pubkey().get_address().as_slice()), 0)],
            };
            let initial_consensus_state = ConsensusState {
                state_id: proof0.commitment().new_state_id,
                timestamp: proof0.commitment().timestamp,
            };

            let input = InitClientInput {
                client_type: LCP_CLIENT_TYPE.to_string(),
                any_client_state: Any::from(initial_client_state),
                any_consensus_state: Any::from(initial_consensus_state),
                current_timestamp: 0,
            };
        }

        let proof1 = {
            let header = MockHeader::new(Height::new(0, 2));
            let any_header = Any::from(AnyHeader::Mock(header));

            let input = UpdateClientInput {
                client_id: proof0.commitment().client_id,
                any_header,
                current_timestamp: 0,
            };

            let res = router::dispatch::<_, LocalLightClientRegistry>(
                Some(&ek),
                &mut lcp_store,
                Command::LightClient(LightClientCommand::UpdateClient(input)),
            );
            assert!(res.is_ok(), "res={:?}", res);
            assert_eq!(lcp_store.revision, 3);
            if let CommandResult::LightClient(LightClientResult::UpdateClient(
                UpdateClientResult(proof),
            )) = res.unwrap()
            {
                proof
            } else {
                unreachable!()
            }
        };
    }
}
