use crate::client_state::ClientState;
use crate::consensus_state::ConsensusState;
use crate::errors::Error;
use crate::message::{
    ClientMessage, CommitmentProofs, RegisterEnclaveKeyMessage, UpdateOperatorsMessage,
};
use attestation_report::{EndorsedAttestationVerificationReport, ReportData};
use crypto::{verify_signature_address, Address, Keccak256};
use hex_literal::hex;
use light_client::commitments::{
    CommitmentPrefix, EthABIEncoder, MisbehaviourProxyMessage, ProxyMessage,
    UpdateStateProxyMessage, VerifyMembershipProxyMessage,
};
use light_client::types::{ClientId, Height, Time};
use light_client::{HostClientKeeper, HostClientReader};
use tiny_keccak::Keccak;

pub const LCP_CLIENT_TYPE: &str = "0000-lcp";

/// keccak256(
///     abi.encode(
///         keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract,bytes32 salt)"),
///         keccak256("LCPClient"),
///         keccak256("1"),
///         0,
///         address(0),
///         0
///     )
/// )
pub const LCP_CLIENT_DOMAIN_SEPARATOR: [u8; 32] =
    hex!("7fd21c2453e80741907e7ff11fd62ae1daa34c6fc0c2eced821f1c1d3fe88a4c");

/// LCPClient is a PoC implementation of LCP Client
/// This is aimed to testing purposes only for now
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LCPClient;

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct EKOperatorInfo {
    expired_at: u64,
    operator: Address,
}

impl EKOperatorInfo {
    fn new(expired_at: u64, operator: Address) -> Self {
        Self {
            expired_at,
            operator,
        }
    }
}

#[allow(clippy::too_many_arguments)]
impl LCPClient {
    /// client_type returns the client type
    pub fn client_type(&self) -> String {
        LCP_CLIENT_TYPE.to_owned()
    }

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
        // operators_threshold_denominator and operators_threshold_numerator must not be 0
        assert!(
            client_state.operators.is_empty()
                || client_state.operators_threshold_denominator != 0
                    && client_state.operators_threshold_numerator != 0
        );
        // check if the operators order is sorted
        client_state.operators.windows(2).for_each(|pair| {
            assert!(pair[0].0 < pair[1].0);
        });
        // operators_threshold_numerator must be less than or equal to operators_threshold_denominator
        assert!(
            client_state.operators_threshold_numerator
                <= client_state.operators_threshold_denominator
        );
        // operators_nonce must be 0
        assert!(client_state.operators_nonce == 0);

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
                ProxyMessage::UpdateState(pmsg) => {
                    self.update_state(ctx, client_id, client_state, pmsg, msg.signatures)
                }
                ProxyMessage::Misbehaviour(pmsg) => {
                    self.submit_misbehaviour(ctx, client_id, client_state, pmsg, msg.signatures)
                }
                _ => Err(Error::unexpected_header_type(format!("{:?}", msg))),
            },
            ClientMessage::RegisterEnclaveKey(msg) => {
                self.register_enclave_key(ctx, client_id, client_state, msg)
            }
            ClientMessage::UpdateOperators(msg) => {
                self.update_operators(ctx, client_id, client_state, msg)
            }
        }
    }

    fn update_state(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        message: UpdateStateProxyMessage,
        signatures: Vec<Vec<u8>>,
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

        self.verify_ek_signatures(
            ctx,
            &client_id,
            &client_state,
            ProxyMessage::from(message.clone()).to_bytes().as_slice(),
            signatures,
        )?;

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

        assert!(!client_state.frozen);

        let (report_data, attestation_time) =
            verify_report(ctx.host_timestamp(), &client_state, &message.report)?;

        let operator = if let Some(operator_signature) = message.operator_signature {
            verify_signature_address(
                compute_eip712_register_enclave_key(&message.report.avr).as_ref(),
                operator_signature.as_ref(),
            )?
        } else {
            Default::default()
        };
        let expected_operator = report_data.operator();
        // check if the operator matches the expected operator in the report data
        assert!(expected_operator.is_zero() || operator == expected_operator);
        self.set_enclave_operator_info(
            ctx,
            &client_id,
            report_data.enclave_key(),
            EKOperatorInfo::new(
                (attestation_time + client_state.key_expiration)?.as_unix_timestamp_secs(),
                operator,
            ),
        );
        Ok(())
    }

    fn update_operators(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        message: UpdateOperatorsMessage,
    ) -> Result<(), Error> {
        // TODO return an error instead of assertion

        assert!(!client_state.frozen);

        assert_eq!(message.nonce, client_state.operators_nonce + 1);

        let sign_bytes = compute_eip712_update_operators(
            client_id.clone(),
            message.nonce,
            message.new_operators.clone(),
            message.new_operators_threshold_numerator,
            message.new_operators_threshold_denominator,
        );

        let mut success = 0u64;
        for (op, sig) in client_state
            .operators
            .clone()
            .into_iter()
            .zip(message.signatures.iter())
            .filter(|(_, sig)| !sig.is_empty())
        {
            // check if the operator's signature is valid
            let operator = verify_signature_address(sign_bytes.as_ref(), sig.as_ref())?;
            assert_eq!(op, operator);
            success += 1;
        }
        assert!(
            success * client_state.operators_threshold_denominator
                >= message.new_operators_threshold_numerator * client_state.operators.len() as u64
        );

        let new_client_state = client_state.with_operators(
            message.new_operators,
            message.nonce,
            message.new_operators_threshold_numerator,
            message.new_operators_threshold_denominator,
        );
        ctx.store_any_client_state(client_id, new_client_state.into())?;

        Ok(())
    }

    fn submit_misbehaviour(
        &self,
        ctx: &mut dyn HostClientKeeper,
        client_id: ClientId,
        client_state: ClientState,
        message: MisbehaviourProxyMessage,
        signatures: Vec<Vec<u8>>,
    ) -> Result<(), Error> {
        message.validate()?;

        assert!(!client_state.frozen);

        for state in message.prev_states.iter() {
            // check if the previous consensus state exists in the store
            let prev_consensus_state: ConsensusState =
                ctx.consensus_state(&client_id, &state.height)?.try_into()?;
            assert!(prev_consensus_state.state_id == state.state_id);
        }

        // check if proxy's validation context matches our's context
        message.context.validate(ctx.host_timestamp())?;
        let sign_bytes = ProxyMessage::from(message).to_bytes();
        self.verify_ek_signatures(ctx, &client_id, &client_state, &sign_bytes, signatures)?;

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
        let commitment_proofs = CommitmentProofs::ethabi_decode(proof.as_slice()).unwrap();
        let msg: VerifyMembershipProxyMessage = commitment_proofs.message()?.try_into()?;

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

        let client_state = ClientState::try_from(ctx.client_state(&client_id)?)?;

        self.verify_ek_signatures(
            ctx,
            &client_id,
            &client_state,
            &commitment_proofs.message,
            commitment_proofs.signatures,
        )?;

        Ok(())
    }

    fn verify_ek_signatures<T: HostClientReader + ?Sized>(
        &self,
        ctx: &T,
        client_id: &ClientId,
        client_state: &ClientState,
        sign_bytes: &[u8],
        signatures: Vec<Vec<u8>>,
    ) -> Result<(), Error> {
        if client_state.operators.is_empty() {
            assert!(signatures.len() == 1);
            let ek = verify_signature_address(sign_bytes, &signatures[0])?;
            assert!(self.is_active_enclave_key(ctx, client_id, ek));
        } else {
            let mut success = 0u64;
            for (signature, operator) in signatures
                .into_iter()
                .zip(client_state.operators.clone().into_iter())
                .filter(|(sig, _)| !sig.is_empty())
            {
                // check if the `header.signer` matches the commitment prover
                let ek = verify_signature_address(sign_bytes, &signature)?;
                // check if the specified signer exists in the client state
                assert!(self.is_active_enclave_key_and_check_operator(ctx, client_id, ek, operator));
                success += 1;
            }
            assert!(
                success * client_state.operators_threshold_denominator
                    >= client_state.operators_threshold_numerator
                        * client_state.operators.len() as u64
            );
        }
        Ok(())
    }

    fn is_active_enclave_key_and_check_operator<T: HostClientReader + ?Sized>(
        &self,
        ctx: &T,
        client_id: &ClientId,
        ek: Address,
        operator: Address,
    ) -> bool {
        let info = match self.get_enclave_operator_info(ctx, client_id, ek) {
            Some(info) => info,
            None => return false,
        };
        assert!(info.operator == operator);
        ctx.host_timestamp().as_unix_timestamp_secs() < info.expired_at
    }

    fn is_active_enclave_key<T: HostClientReader + ?Sized>(
        &self,
        ctx: &T,
        client_id: &ClientId,
        ek: Address,
    ) -> bool {
        let info = match self.get_enclave_operator_info(ctx, client_id, ek) {
            Some(info) => info,
            None => return false,
        };
        ctx.host_timestamp().as_unix_timestamp_secs() < info.expired_at
    }

    fn set_enclave_operator_info<T: HostClientKeeper + ?Sized>(
        &self,
        ctx: &mut T,
        client_id: &ClientId,
        ek: Address,
        info: EKOperatorInfo,
    ) {
        match self.get_enclave_operator_info(ctx, client_id, ek) {
            Some(v) => {
                assert!(v.expired_at == info.expired_at && v.operator == info.operator);
            }
            None => {
                ctx.set(
                    enclave_key_path(client_id, ek),
                    serde_json::to_string(&info).unwrap().into_bytes(),
                );
            }
        }
    }

    fn get_enclave_operator_info<T: HostClientReader + ?Sized>(
        &self,
        ctx: &T,
        client_id: &ClientId,
        ek: Address,
    ) -> Option<EKOperatorInfo> {
        let info = ctx.get(enclave_key_path(client_id, ek).as_slice())?;
        Some(serde_json::from_slice(info.as_slice()).unwrap())
    }
}

pub fn compute_eip712_register_enclave_key(avr: &str) -> Vec<u8> {
    // 0x1901 | DOMAIN_SEPARATOR_REGISTER_ENCLAVE_KEY | keccak256(keccak256("RegisterEnclaveKey(string avr)") | keccak256(avr))
    let type_hash = {
        let mut h = Keccak::new_keccak256();
        h.update(&keccak256(b"RegisterEnclaveKey(string avr)"));
        h.update(&keccak256(avr.as_bytes()));
        let mut result = [0u8; 32];
        h.finalize(result.as_mut());
        result
    };
    [0x19, 0x01]
        .into_iter()
        .chain(LCP_CLIENT_DOMAIN_SEPARATOR.into_iter())
        .chain(type_hash.into_iter())
        .collect()
}

pub fn compute_eip712_register_enclave_key_hash(avr: &str) -> [u8; 32] {
    keccak256(&compute_eip712_register_enclave_key(avr))
}

pub fn compute_eip712_update_operators(
    client_id: ClientId,
    nonce: u64,
    new_operators: Vec<Address>,
    threshold_numerator: u64,
    threshold_denominator: u64,
) -> Vec<u8> {
    // 0x1901 | DOMAIN_SEPARATOR_UPDATE_OPERATORS | keccak256(keccak256("UpdateOperators(string clientId,uint64 nonce,address[] newOperators,uint64 thresholdNumerator,uint64 thresholdDenominator)") | keccak256(client_id) | nonce | keccak256(new_operators) | threshold_numerator | threshold_denominator)
    let type_hash = {
        let mut h = Keccak::new_keccak256();
        h.update(&keccak256(b"UpdateOperators(string clientId,uint64 nonce,address[] newOperators,uint64 thresholdNumerator,uint64 thresholdDenominator)"));
        h.update(&keccak256(client_id.as_bytes()));
        h.update(&nonce.to_be_bytes());
        h.update(&keccak256(
            new_operators
                .iter()
                .fold(Vec::new(), |mut acc, x| {
                    acc.extend_from_slice(x.0.as_ref());
                    acc
                })
                .as_ref(),
        ));
        h.update(&threshold_numerator.to_be_bytes());
        h.update(&threshold_denominator.to_be_bytes());
        let mut result = [0u8; 32];
        h.finalize(result.as_mut());
        result
    };
    [0x19, 0x01]
        .into_iter()
        .chain(LCP_CLIENT_DOMAIN_SEPARATOR.into_iter())
        .chain(type_hash.into_iter())
        .collect()
}

pub fn compute_eip712_update_operators_hash(
    client_id: ClientId,
    nonce: u64,
    new_operators: Vec<Address>,
    threshold_numerator: u64,
    threshold_denominator: u64,
) -> [u8; 32] {
    keccak256(&compute_eip712_update_operators(
        client_id,
        nonce,
        new_operators,
        threshold_numerator,
        threshold_denominator,
    ))
}

// verify_report
// - verifies the Attestation Verification Report
// - calculate a key expiration with client_state and report's timestamp
fn verify_report(
    current_timestamp: Time,
    client_state: &ClientState,
    eavr: &EndorsedAttestationVerificationReport,
) -> Result<(ReportData, Time), Error> {
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

    let report_data = quote.report_data();
    report_data.validate()?;
    Ok((report_data, quote.attestation_time))
}

fn enclave_key_path(client_id: &ClientId, ek: Address) -> Vec<u8> {
    format!("clients/{}/aux/enclave_keys/{}", client_id, ek)
        .as_bytes()
        .to_vec()
}

fn keccak256(bz: &[u8]) -> [u8; 32] {
    let mut keccak = Keccak::new_keccak256();
    let mut result = [0u8; 32];
    keccak.update(bz);
    keccak.finalize(result.as_mut());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::UpdateClientMessage;
    use alloc::rc::Rc;
    use alloc::sync::Arc;
    use attestation_report::{AttestationVerificationReport, ReportData};
    use context::Context;
    use core::cell::RefCell;
    use core::str::FromStr;
    use core::time::Duration;
    use crypto::{EnclaveKey, EnclavePublicKey, Signer};
    use ibc::{
        mock::{
            client_state::MockClientState, consensus_state::MockConsensusState, header::MockHeader,
            misbehaviour::Misbehaviour as MockMisbehaviour,
        },
        Height as ICS02Height,
    };
    use light_client::{commitments::prove_commitment, UpdateClientResult};
    use light_client::{ClientKeeper, LightClient, LightClientResolver, MapLightClientRegistry};
    use mock_lc::MockLightClient;
    use sgx_types::{sgx_quote_t, sgx_report_body_t};
    use store::memory::MemStore;

    #[test]
    fn test_compute_eip712_register_enclave_key() {
        let avr = "{}";
        let expected = hex!("2ab70eb55dea90c4d477a7e668812653ca37c079036e92e31d4d092bcacf61cb");
        let got = compute_eip712_register_enclave_key_hash(avr);
        assert_eq!(got, expected);
    }

    #[test]
    fn test_client() {
        // ek is a signing key to prove LCP's commitments
        let ek = EnclaveKey::new().unwrap();
        // lcp_store is a store to keeps LCP's state
        let lcp_store = Rc::new(RefCell::new(MemStore::default()));
        // ibc_store is a store to keeps downstream's state
        let ibc_store = Rc::new(RefCell::new(MemStore::default()));

        // pseudo operator key
        type OperatorKey = EnclaveKey;
        let op_key = OperatorKey::new().unwrap();

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
                ..Default::default()
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
            let report = generate_dummy_eavr(&ek.get_pubkey());
            let operator_signature = op_key
                .sign(compute_eip712_register_enclave_key(report.avr.as_str()).as_slice())
                .unwrap();
            let header = ClientMessage::RegisterEnclaveKey(RegisterEnclaveKeyMessage {
                report,
                operator_signature: Some(operator_signature),
            });
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

            let res = prove_commitment(ctx.get_enclave_key(), res.message.into());
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
                signatures: vec![proof1.signature],
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
            let res = prove_commitment(ctx.get_enclave_key(), data.message.into());
            assert!(res.is_ok(), "res={:?}", res);
            res.unwrap()
        };

        // 7. on the downstream side, updates LCP Light Client's state with the message from the ELC
        {
            let header = ClientMessage::UpdateClient(UpdateClientMessage {
                proxy_message: misbehaviour_proof.message().unwrap(),
                signatures: vec![misbehaviour_proof.signature],
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
                report_data: ReportData::new(key.as_address(), None).into(),
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
