mod client;
mod errors;
#[cfg(test)]
mod tests {
    use super::client::register_implementations;
    use crate::header::Header;
    use crate::header::UpdateClientHeader;
    use crate::{client_state::ClientState, consensus_state::ConsensusState, crypto::Address};
    use core::time::Duration;
    use crypto::EnclaveKey;
    use enclave_commands::CommandParams;
    use enclave_commands::{
        Command, CommandResult, EnclaveCommand, InitClientInput, InitClientResult,
        LightClientCommand, LightClientResult, UpdateClientInput, UpdateClientResult,
    };
    use handler::router;
    use ibc::{
        core::ics02_client::{
            client_consensus::AnyConsensusState, client_state::AnyClientState, header::AnyHeader,
        },
        mock::{
            client_state::{MockClientState, MockConsensusState},
            header::MockHeader,
        },
        Height as ICS02Height,
    };
    use lazy_static::lazy_static;
    use lcp_types::{Any, Height};
    use light_client::{LightClient, LightClientRegistry, LightClientSource};
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;
    use store::memory::MemStore;
    use tempdir::TempDir;

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
    fn test_ibc_client() {
        // ek is a signing key to prove LCP's commitments
        let ek = EnclaveKey::new().unwrap();
        // lcp_store is a store to keeps LCP's state
        let mut lcp_store = MemStore::new();
        // ibc_store is a store to keeps downstream's state
        let mut ibc_store = MemStore::new();

        let tmp_dir = TempDir::new("lcp").unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();

        // 1. initializes Light Client(Mock) corresponding to the upstream chain on the LCP side
        let proof0 = {
            let header = MockHeader::new(ICS02Height::new(0, 1).unwrap());
            let client_state = AnyClientState::Mock(MockClientState::new(header));
            let consensus_state = AnyConsensusState::Mock(MockConsensusState::new(header));

            let input = InitClientInput {
                any_client_state: Any::from(client_state).into(),
                any_consensus_state: Any::from(consensus_state).into(),
                current_timestamp: 0,
            };
            assert_eq!(lcp_store.revision, 1);
            let res = router::dispatch::<_, LocalLightClientRegistry>(
                Some(&ek),
                &mut lcp_store,
                EnclaveCommand::new(
                    CommandParams::new(home.clone()),
                    Command::LightClient(LightClientCommand::InitClient(input)),
                ),
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

        // 2. initializes Light Client for LCP on the downstream side
        let lcp_client_id = {
            let expired_at = SystemTime::now()
                .checked_add(Duration::from_secs(60))
                .unwrap()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let initial_client_state = ClientState {
                latest_height: Height::new(0, 1),
                mr_enclave: Default::default(),
                key_expiration: Duration::from_secs(60).as_nanos(),
                keys: vec![(
                    expired_at,
                    Address::from(ek.get_pubkey().get_address().as_slice()),
                )],
            };
            let initial_consensus_state = ConsensusState {
                state_id: proof0.commitment().new_state_id,
                timestamp: proof0.commitment().timestamp,
            };

            let input = InitClientInput {
                any_client_state: initial_client_state.into(),
                any_consensus_state: initial_consensus_state.into(),
                current_timestamp: 0,
            };
            let res = router::dispatch::<_, LocalLightClientRegistry>(
                Some(&ek),
                &mut ibc_store,
                EnclaveCommand::new(
                    CommandParams::new(home.clone()),
                    Command::LightClient(LightClientCommand::InitClient(input)),
                ),
            );
            assert!(res.is_ok(), "res={:?}", res);
            if let CommandResult::LightClient(LightClientResult::InitClient(InitClientResult(
                proof,
            ))) = res.unwrap()
            {
                proof.commitment().client_id
            } else {
                unreachable!()
            }
        };

        // 3. updates the Light Client state on the LCP side
        let proof1 = {
            let header = MockHeader::new(ICS02Height::new(0, 2).unwrap());
            let input = UpdateClientInput {
                client_id: proof0.commitment().client_id,
                any_header: Any::from(AnyHeader::Mock(header)).into(),
                current_timestamp: 0,
            };

            let res = router::dispatch::<_, LocalLightClientRegistry>(
                Some(&ek),
                &mut lcp_store,
                EnclaveCommand::new(
                    CommandParams::new(home.clone()),
                    Command::LightClient(LightClientCommand::UpdateClient(input)),
                ),
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

        // 4. on the downstream side, updates LCP Light Client's state with the commitment from the LCP
        {
            let header = Header::UpdateClient(UpdateClientHeader {
                commitment: proof1.commitment(),
                commitment_bytes: proof1.commitment_bytes,
                signer: proof1.signer,
                signature: proof1.signature,
            });
            let input = UpdateClientInput {
                client_id: lcp_client_id.clone(),
                any_header: header.into(),
                current_timestamp: SystemTime::now()
                    .checked_add(Duration::from_secs(60))
                    .unwrap()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
            };
            let res = router::dispatch::<_, LocalLightClientRegistry>(
                Some(&ek),
                &mut ibc_store,
                EnclaveCommand::new(
                    CommandParams::new(home.clone()),
                    Command::LightClient(LightClientCommand::UpdateClient(input)),
                ),
            );
            assert!(res.is_ok(), "res={:?}", res);
        }
    }
}
