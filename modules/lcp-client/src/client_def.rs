use crate::client_state::ClientState;
use crate::consensus_state::ConsensusState;
use crate::errors::Error;
use crate::message::{ClientMessage, RegisterEnclaveKeyMessage};
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::{verify_signature_address, Address, Keccak256};
use light_client::commitments::{
    CommitmentPrefix, CommitmentProof, EthABIEncoder, MisbehaviourProxyMessage, ProxyMessage,
    UpdateStateProxyMessage, VerifyMembershipProxyMessage,
};
use light_client::types::{ClientId, Height, Time};
use light_client::{ClientKeeper, ClientReader, HostClientKeeper, HostClientReader};

pub const LCP_CLIENT_TYPE: &str = "0000-lcp";

/// LCPClient is a PoC implementation of LCP Client
/// This is aimed to testing purposes only for now
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LCPClient;

#[allow(clippy::too_many_arguments)]
impl LCPClient {
    /// initialse initialises a client state with an initial client state and consensus state
    pub fn initialise(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        consensus_state: ConsensusState,
    ) -> Result<(), Error> {
        // key_expiration must not be 0
        assert!(!client_state.key_expiration.is_zero());
        // An initial client state's latest height must be empty
        assert!(client_state.latest_height.is_zero());
        // mr_enclave length must be 32
        assert!(client_state.mr_enclave.len() == 32);
        // An initial consensus state must be empty
        assert!(consensus_state.is_empty());

        ctx.store_any_client_state(client_id.clone(), client_state.clone().into())?;
        ctx.store_any_consensus_state(
            client_id,
            client_state.latest_height,
            consensus_state.into(),
        )?;
        Ok(())
    }

    // verify_client_message verifies a client message
    pub fn update_client(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        message: ClientMessage,
    ) -> Result<(), Error> {
        let client_state = ctx.client_state(&client_id)?.try_into()?;
        match message {
            ClientMessage::UpdateClient(msg) => match msg.proxy_message {
                ProxyMessage::UpdateState(pmsg) => self.update_state(
                    ctx,
                    client_id,
                    client_state,
                    pmsg,
                    msg.signer,
                    msg.signature,
                ),
                ProxyMessage::Misbehaviour(pmsg) => self.submit_misbehaviour(
                    ctx,
                    client_id,
                    client_state,
                    pmsg,
                    msg.signer,
                    msg.signature,
                ),
                _ => Err(Error::unexpected_header_type(format!("{:?}", msg))),
            },
            ClientMessage::RegisterEnclaveKey(msg) => {
                self.register_enclave_key(ctx, client_id, client_state, msg)
            }
        }
    }

    fn update_state(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        message: UpdateStateProxyMessage,
        signer: Address,
        signature: Vec<u8>,
    ) -> Result<(), Error> {
        message.validate()?;
        // TODO return an error instead of assertion

        assert!(!client_state.frozen);

        if client_state.latest_height.is_zero() {
            // if the client state's latest height is zero, the commitment's new_state must be non-nil
            assert!(!message.emitted_states.is_empty());
        } else {
            // if the client state's latest height is non-zero, the commitment's prev_* must be non-nil
            assert!(message.prev_height.is_some() && message.prev_state_id.is_some());
            // check if the previous consensus state exists in the store
            let prev_consensus_state: ConsensusState = ctx
                .consensus_state(&client_id, &message.prev_height.unwrap())?
                .try_into()?;
            assert!(prev_consensus_state.state_id == message.prev_state_id.unwrap());
        }

        // check if the specified signer exists in the client state
        assert!(self.contains_enclave_key(ctx, &client_id, signer));

        // check if the `header.signer` matches the commitment prover
        let signer2 = verify_signature_address(
            ProxyMessage::from(message.clone()).to_bytes().as_slice(),
            &signature,
        )
        .unwrap();
        assert!(signer == signer2);

        // check if proxy's validation context matches our's context
        message.context.validate(ctx.host_timestamp())?;

        // create a new state
        let new_client_state = client_state.with_header(&message);
        let new_consensus_state = ConsensusState {
            state_id: message.post_state_id,
            timestamp: message.timestamp,
        };

        ctx.store_any_client_state(client_id.clone(), new_client_state.into())?;
        ctx.store_any_consensus_state(client_id, message.post_height, new_consensus_state.into())?;
        Ok(())
    }

    fn register_enclave_key(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        message: RegisterEnclaveKeyMessage,
    ) -> Result<(), Error> {
        // TODO return an error instead of assertion

        let eavr = message.0;
        let (key, attestation_time) = verify_report(ctx.host_timestamp(), &client_state, &eavr)?;

        self.add_enclave_key(
            ctx,
            &client_id,
            key,
            (attestation_time + client_state.key_expiration)?.as_unix_timestamp_secs(),
        );
        Ok(())
    }

    fn submit_misbehaviour(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        message: MisbehaviourProxyMessage,
        signer: Address,
        signature: Vec<u8>,
    ) -> Result<(), Error> {
        message.validate()?;

        assert!(!client_state.frozen);

        for state in message.prev_states.iter() {
            // check if the previous consensus state exists in the store
            let prev_consensus_state: ConsensusState =
                ctx.consensus_state(&client_id, &state.height)?.try_into()?;
            assert!(prev_consensus_state.state_id == state.state_id);
        }

        // check if the specified signer exists in the client state
        assert!(self.contains_enclave_key(ctx, &client_id, signer));

        // check if proxy's validation context matches our's context
        message.context.validate(ctx.host_timestamp())?;

        // check if the `header.signer` matches the commitment prover
        let signer2 = verify_signature_address(
            ProxyMessage::from(message).to_bytes().as_slice(),
            &signature,
        )?;
        assert!(signer == signer2);

        let new_client_state = client_state.with_frozen();
        ctx.store_any_client_state(client_id, new_client_state.into())?;

        Ok(())
    }

    /// verify_membership is a generic proof verification method which verifies a proof of the existence of a value at a given path at the specified height.
    pub fn verify_membership(
        &self,
        ctx: &dyn HostClientReader,
        client_id: ClientId,
        prefix: CommitmentPrefix,
        path: String,
        value: Vec<u8>,
        proof_height: Height,
        proof: Vec<u8>,
    ) -> Result<(), Error> {
        // TODO return an error instead of assertion

        // convert `proof` to CommitmentProof
        let commitment_proof = CommitmentProof::ethabi_decode(proof.as_slice()).unwrap();
        let msg: VerifyMembershipProxyMessage = commitment_proof.message()?.try_into()?;

        // check if `.prefix` matches the counterparty connection's prefix
        assert!(msg.prefix == prefix);
        // check if `.path` matches expected the commitment path
        assert!(msg.path == path);
        // check if `.height` matches proof height
        assert!(msg.height == proof_height);

        // check if `.value` matches expected state
        assert!(msg.value == Some(value.keccak256()));

        // check if `.state_id` matches the corresponding stored consensus state's state_id
        let consensus_state =
            ConsensusState::try_from(ctx.consensus_state(&client_id, &proof_height)?)?;
        assert!(consensus_state.state_id == msg.state_id);

        // check if the `commitment_proof.signer` matches the commitment prover
        let signer =
            verify_signature_address(&commitment_proof.message, &commitment_proof.signature)?;
        assert!(commitment_proof.signer == signer);

        // check if the specified signer is not expired and exists in the client state
        assert!(self.is_active_enclave_key(ctx, &client_id, signer));

        Ok(())
    }

    pub fn client_type(&self) -> String {
        LCP_CLIENT_TYPE.to_owned()
    }

    fn contains_enclave_key<T: ClientReader + ?Sized>(
        &self,
        ctx: &T,
        client_id: &ClientId,
        key: Address,
    ) -> bool {
        ctx.get(enclave_key_path(client_id, key).as_slice())
            .is_some()
    }

    fn is_active_enclave_key<T: HostClientReader + ?Sized>(
        &self,
        ctx: &T,
        client_id: &ClientId,
        key: Address,
    ) -> bool {
        let expired_at = match ctx.get(enclave_key_path(client_id, key).as_slice()) {
            Some(bz) => u64::from_be_bytes(bz.as_slice().try_into().unwrap()),
            None => return false,
        };
        ctx.host_timestamp().as_unix_timestamp_secs() < expired_at
    }

    fn add_enclave_key<T: ClientKeeper + ?Sized>(
        &self,
        ctx: &mut T,
        client_id: &ClientId,
        key: Address,
        expired_at: u64,
    ) {
        ctx.set(
            enclave_key_path(client_id, key),
            expired_at.to_be_bytes().to_vec(),
        );
    }
}

// verify_report
// - verifies the Attestation Verification Report
// - calculate a key expiration with client_state and report's timestamp
fn verify_report(
    current_timestamp: Time,
    client_state: &ClientState,
    eavr: &EndorsedAttestationVerificationReport,
) -> Result<(Address, Time), Error> {
    // verify AVR with Intel SGX Attestation Report Signing CA
    // NOTE: This verification is skipped in tests because the CA is not available in the test environment
    #[cfg(not(test))]
    attestation_report::verify_report(current_timestamp, eavr)?;

    let quote = eavr.get_avr()?.parse_quote()?;

    // check if attestation report's timestamp is not expired
    let key_expiration = (quote.attestation_time + client_state.key_expiration)?;
    if current_timestamp > key_expiration {
        return Err(Error::expired_avr(
            current_timestamp,
            quote.attestation_time,
            client_state.key_expiration,
        ));
    }

    // check if `mr_enclave` that is included in the quote matches the expected value
    if quote.raw.report_body.mr_enclave.m != client_state.mr_enclave.as_slice() {
        return Err(Error::mrenclave_mismatch(
            quote.raw.report_body.mr_enclave.m.to_vec(),
            client_state.mr_enclave.clone(),
        ));
    }

    Ok((quote.get_enclave_key_address()?, quote.attestation_time))
}

fn enclave_key_path(client_id: &ClientId, key: Address) -> Vec<u8> {
    format!("clients/{}/aux/enclave_keys/{}", client_id, key)
        .as_bytes()
        .to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::UpdateClientMessage;
    use alloc::rc::Rc;
    use alloc::sync::Arc;
    use attestation_report::AttestationVerificationReport;
    use context::Context;
    use core::cell::RefCell;
    use core::str::FromStr;
    use core::time::Duration;
    use crypto::{EnclaveKey, EnclavePublicKey};
    use ibc::{
        mock::{
            client_state::MockClientState, consensus_state::MockConsensusState, header::MockHeader,
            misbehaviour::Misbehaviour as MockMisbehaviour,
        },
        Height as ICS02Height,
    };
    use light_client::{commitments::prove_commitment, UpdateClientResult};
    use light_client::{LightClient, LightClientResolver, MapLightClientRegistry};
    use mock_lc::MockLightClient;
    use sgx_types::{sgx_quote_t, sgx_report_body_t};
    use store::memory::MemStore;

    #[test]
    fn test_client() {
        // ek is a signing key to prove LCP's commitments
        let ek = EnclaveKey::new().unwrap();
        // lcp_store is a store to keeps LCP's state
        let lcp_store = Rc::new(RefCell::new(MemStore::default()));
        // ibc_store is a store to keeps downstream's state
        let ibc_store = Rc::new(RefCell::new(MemStore::default()));

        let registry = build_lc_registry();
        let lcp_client = LCPClient::default();
        let mock_client = MockLightClient::default();

        // 1. initializes Light Client for LCP on the downstream side
        let lcp_client_id = {
            let expired_at = (Time::now() + Duration::from_secs(60)).unwrap();
            let initial_client_state = ClientState {
                mr_enclave: [0u8; 32].to_vec(),
                key_expiration: Duration::from_secs(60 * 60 * 24 * 7),
                frozen: false,
                latest_height: Height::zero(),
            };
            let initial_consensus_state = ConsensusState {
                state_id: Default::default(),
                timestamp: Time::unix_epoch(),
            };

            let mut ctx = Context::new(registry.clone(), ibc_store.clone(), &ek);
            ctx.set_timestamp(Time::now());

            let client_id = ClientId::from_str(&format!("{}-0", lcp_client.client_type())).unwrap();

            let res = lcp_client.initialise(
                &mut ctx,
                client_id.clone(),
                initial_client_state,
                initial_consensus_state,
            );
            assert!(res.is_ok(), "res={:?}", res);
            client_id
        };

        // 2. register enclave key to the LCP client
        {
            let mut ctx = Context::new(registry.clone(), ibc_store.clone(), &ek);
            ctx.set_timestamp(Time::now());
            let header = ClientMessage::RegisterEnclaveKey(RegisterEnclaveKeyMessage(
                generate_dummy_eavr(&ek.get_pubkey()),
            ));
            let res = lcp_client.update_client(&mut ctx, lcp_client_id.clone(), header);
            assert!(res.is_ok(), "res={:?}", res);
        }

        // 3. initializes Light Client(Mock) corresponding to the upstream chain on the LCP side
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

        // 4. updates the Light Client state on the LCP side
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

            let res = match res.unwrap() {
                UpdateClientResult::UpdateState(res) => res,
                _ => panic!("unexpected result"),
            };
            let (client_state, consensus_state, height) = {
                (
                    res.new_any_client_state,
                    res.new_any_consensus_state,
                    res.height,
                )
            };

            let res = prove_commitment(
                ctx.get_enclave_key(),
                ctx.get_enclave_key().pubkey().unwrap().as_address(),
                res.message.into(),
            );
            assert!(res.is_ok(), "res={:?}", res);

            ctx.store_any_client_state(upstream_client_id.clone(), client_state)
                .unwrap();
            ctx.store_any_consensus_state(upstream_client_id.clone(), height, consensus_state)
                .unwrap();
            res.unwrap()
        };

        // 5. on the downstream side, updates LCP Light Client's state with the message from the ELC
        {
            let header = ClientMessage::UpdateClient(UpdateClientMessage {
                proxy_message: proof1.message().unwrap(),
                signer: proof1.signer,
                signature: proof1.signature,
            });
            let mut ctx = Context::new(registry.clone(), ibc_store.clone(), &ek);
            ctx.set_timestamp((Time::now() + Duration::from_secs(60)).unwrap());

            let res = lcp_client.update_client(&mut ctx, lcp_client_id.clone(), header);
            assert!(res.is_ok(), "res={:?}", res);
        }

        // 6. on the upstream side, updates the Light Client state with a misbehaviour
        let misbehaviour_proof = {
            let mut ctx = Context::new(registry.clone(), lcp_store, &ek);
            ctx.set_timestamp(Time::now());

            let mock_misbehaviour = MockMisbehaviour {
                client_id: upstream_client_id.clone().into(),
                header1: MockHeader::new(ICS02Height::new(0, 3).unwrap()),
                header2: MockHeader::new(ICS02Height::new(0, 3).unwrap()),
            };
            let res = mock_client
                .update_client(
                    &ctx,
                    upstream_client_id,
                    mock_lc::Misbehaviour::from(mock_misbehaviour).into(),
                )
                .unwrap();
            let data = match res {
                UpdateClientResult::Misbehaviour(data) => data,
                _ => unreachable!(),
            };
            let res = prove_commitment(
                ctx.get_enclave_key(),
                ctx.get_enclave_key().pubkey().unwrap().as_address(),
                data.message.into(),
            );
            assert!(res.is_ok(), "res={:?}", res);
            res.unwrap()
        };

        // 7. on the downstream side, updates LCP Light Client's state with the message from the ELC
        {
            let header = ClientMessage::UpdateClient(UpdateClientMessage {
                proxy_message: misbehaviour_proof.message().unwrap(),
                signer: misbehaviour_proof.signer,
                signature: misbehaviour_proof.signature,
            });
            let mut ctx = Context::new(registry, ibc_store, &ek);
            ctx.set_timestamp((Time::now() + Duration::from_secs(60)).unwrap());

            let res = lcp_client.update_client(&mut ctx, lcp_client_id, header);
            assert!(res.is_ok(), "res={:?}", res);
        }
    }

    fn build_lc_registry() -> Arc<dyn LightClientResolver> {
        let registry = MapLightClientRegistry::new();
        Arc::new(registry)
    }

    fn generate_dummy_eavr(key: &EnclavePublicKey) -> EndorsedAttestationVerificationReport {
        let quote = sgx_quote_t {
            version: 4,
            report_body: sgx_report_body_t {
                report_data: key.as_report_data(),
                ..Default::default()
            },
            ..Default::default()
        };
        // transmute quote to Vec<u8>
        let quote = unsafe {
            core::mem::transmute_copy::<sgx_quote_t, [u8; core::mem::size_of::<sgx_quote_t>()]>(
                &quote,
            )
        };
        let now = chrono::Utc::now();
        let attr = AttestationVerificationReport {
            id: "23856791181030202675484781740313693463".to_string(),
            // TODO refactoring
            timestamp: format!(
                "{}000",
                now.format("%Y-%m-%dT%H:%M:%S%.f%z")
                    .to_string()
                    .strip_suffix("+0000")
                    .unwrap()
            ),
            version: 4,
            advisory_url: "https://security-center.intel.com".to_string(),
            // advisory_ids,
            // isv_enclave_quote_status,
            platform_info_blob: None,
            isv_enclave_quote_body: base64::encode(&quote.as_slice()[..432]),
            ..Default::default()
        };

        EndorsedAttestationVerificationReport {
            avr: attr.to_canonical_json().unwrap(),
            ..Default::default()
        }
    }
}
