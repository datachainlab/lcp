mod client;
mod errors;
#[cfg(test)]
mod tests {
    use crate::header::{Header, UpdateClientHeader};
    use crate::tests::client::LCPLightClient;
    use crate::{client_state::ClientState, consensus_state::ConsensusState};
    use alloc::rc::Rc;
    use alloc::sync::Arc;
    use commitments::prover::prove_update_client_commitment;
    use context::Context;
    use core::cell::RefCell;
    use core::str::FromStr;
    use core::time::Duration;
    use crypto::{Address, EnclaveKey};
    use ibc::{
        mock::{
            client_state::MockClientState, consensus_state::MockConsensusState, header::MockHeader,
        },
        Height as ICS02Height,
    };
    use lcp_types::{ClientId, Height, Time};
    use light_client::{ClientKeeper, LightClient};
    use light_client_registry::memory::HashMapLightClientRegistry;
    use light_client_registry::LightClientResolver;
    use mock_lc::MockLightClient;
    use store::memory::MemStore;
    use tempdir::TempDir;

    fn build_lc_registry() -> Arc<dyn LightClientResolver> {
        let registry = HashMapLightClientRegistry::new();
        Arc::new(registry)
    }

    #[test]
    fn test_ibc_client() {
        // ek is a signing key to prove LCP's commitments
        let ek = EnclaveKey::new().unwrap();
        // lcp_store is a store to keeps LCP's state
        let lcp_store = Rc::new(RefCell::new(MemStore::default()));
        // ibc_store is a store to keeps downstream's state
        let ibc_store = Rc::new(RefCell::new(MemStore::default()));

        let lcp_client = LCPLightClient::default();
        let mock_client = MockLightClient::default();

        let registry = build_lc_registry();

        let tmp_dir = TempDir::new("lcp").unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();

        // 1. initializes Light Client for LCP on the downstream side
        let lcp_client_id = {
            let expired_at = (Time::now() + Duration::from_secs(60)).unwrap();
            let initial_client_state = ClientState {
                latest_height: Height::zero(),
                mr_enclave: Default::default(),
                key_expiration: Duration::from_secs(60),
                keys: vec![(Address::from(&ek.get_pubkey()), expired_at)],
            };
            let initial_consensus_state = ConsensusState {
                state_id: Default::default(),
                timestamp: Time::unix_epoch(),
            };

            let mut ctx = Context::new(registry.clone(), ibc_store.clone(), &ek);
            ctx.set_timestamp(Time::now());

            let res = lcp_client.create_client(
                &ctx,
                initial_client_state.clone().into(),
                initial_consensus_state.clone().into(),
            );
            assert!(res.is_ok(), "res={:?}", res);

            let client_id = ClientId::from_str(&format!("{}-0", lcp_client.client_type())).unwrap();
            ctx.store_client_type(client_id.clone(), lcp_client.client_type())
                .unwrap();
            ctx.store_any_client_state(client_id.clone(), initial_client_state.into())
                .unwrap();
            ctx.store_any_consensus_state(
                client_id.clone(),
                res.unwrap().height,
                initial_consensus_state.into(),
            )
            .unwrap();
            client_id
        };

        // 2. initializes Light Client(Mock) corresponding to the upstream chain on the LCP side
        let upstream_client_id = {
            let header = MockHeader::new(ICS02Height::new(0, 1).unwrap());
            let client_state = mock_lc::ClientState::from(MockClientState::new(header));
            let consensus_state = mock_lc::ConsensusState::from(MockConsensusState::new(header));
            let mut ctx = Context::new(registry.clone(), lcp_store.clone(), &ek);
            ctx.set_timestamp(Time::now());

            let res = mock_client.create_client(
                &ctx,
                client_state.clone().into(),
                consensus_state.clone().into(),
            );
            assert!(res.is_ok(), "res={:?}", res);

            let client_id =
                ClientId::from_str(&format!("{}-0", mock_client.client_type())).unwrap();
            ctx.store_client_type(client_id.clone(), mock_client.client_type())
                .unwrap();
            ctx.store_any_client_state(client_id.clone(), client_state.into())
                .unwrap();
            ctx.store_any_consensus_state(
                client_id.clone(),
                res.unwrap().height,
                consensus_state.into(),
            )
            .unwrap();
            client_id
        };

        // 3. updates the Light Client state on the LCP side
        let proof1 = {
            let header = MockHeader::new(ICS02Height::new(0, 2).unwrap());

            let mut ctx = Context::new(registry.clone(), lcp_store.clone(), &ek);
            ctx.set_timestamp(Time::now());
            let res = mock_client.update_client(
                &ctx,
                upstream_client_id.clone(),
                mock_lc::Header::from(header).into(),
            );
            assert!(res.is_ok(), "res={:?}", res);
            let res = res.unwrap();
            let (client_state, consensus_state, height) = {
                (
                    res.new_any_client_state,
                    res.new_any_consensus_state,
                    res.height,
                )
            };

            let res = prove_update_client_commitment(ctx.get_enclave_key(), res.commitment);
            assert!(res.is_ok(), "res={:?}", res);

            ctx.store_any_client_state(upstream_client_id.clone(), client_state)
                .unwrap();
            ctx.store_any_consensus_state(upstream_client_id.clone(), height, consensus_state)
                .unwrap();
            res.unwrap()
        };

        // 4. on the downstream side, updates LCP Light Client's state with the commitment from the LCP
        {
            let header = Header::UpdateClient(UpdateClientHeader {
                commitment: proof1.commitment(),
                commitment_bytes: proof1.commitment_bytes,
                signer: proof1.signer,
                signature: proof1.signature,
            });
            let mut ctx = Context::new(registry.clone(), ibc_store.clone(), &ek);
            ctx.set_timestamp((Time::now() + Duration::from_secs(60)).unwrap());

            let res = lcp_client.update_client(&ctx, lcp_client_id, header.into());
            assert!(res.is_ok(), "res={:?}", res);
        }
    }
}
