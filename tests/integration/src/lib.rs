#[cfg(test)]
mod config;
#[cfg(test)]
mod relayer;
#[cfg(test)]
mod types;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relayer::Relayer;
    use anyhow::{anyhow, bail};
    use commitments::UpdateClientCommitment;
    use ecall_commands::{
        CommitmentProofPair, GenerateEnclaveKeyInput, InitClientInput, UpdateClientInput,
        VerifyMembershipInput,
    };
    use enclave_api::{Enclave, EnclaveCommandAPI};
    use host_environment::Environment;
    use ibc::core::{
        ics23_commitment::{commitment::CommitmentProofBytes, merkle::MerkleProof},
        ics24_host::{
            identifier::{ChannelId, PortId},
            path::ChannelEndPath,
            Path,
        },
    };
    use ibc_proto::protobuf::Protobuf;
    use ibc_test_framework::prelude::{
        run_binary_channel_test, BinaryChannelTest, ChainHandle, Config, ConnectedChains,
        ConnectedChannel, Error, RelayerDriver, TestConfig, TestOverrides,
    };
    use keymanager::EnclaveKeyManager;
    use lcp_types::Time;
    use log::*;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use store::{host::HostStore, memory::MemStore};
    use tempfile::TempDir;
    use tokio::runtime::Runtime as TokioRuntime;

    static ENCLAVE_FILE: &str = "../../bin/enclave.signed.so";
    static ENV_SETUP_NODES: &str = "SETUP_NODES";

    struct ELCStateVerificationTest {
        enclave: Enclave<store::memory::MemStore>,
    }

    impl TestOverrides for ELCStateVerificationTest {
        fn modify_relayer_config(&self, _config: &mut Config) {}
    }

    impl BinaryChannelTest for ELCStateVerificationTest {
        fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
            &self,
            _config: &TestConfig,
            _relayer: RelayerDriver,
            chains: ConnectedChains<ChainA, ChainB>,
            _channel: ConnectedChannel<ChainA, ChainB>,
        ) -> Result<(), Error> {
            let rt = Arc::new(TokioRuntime::new()?);
            let config_a = chains.handle_a().config()?;
            let rly = Relayer::new(config_a, rt).unwrap();
            verify(rly, &self.enclave).unwrap();
            Ok(())
        }
    }

    #[test]
    fn test_elc_state_verification() {
        let tmp_dir = TempDir::new().unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();
        host::set_environment(Environment::new(
            home.into(),
            Arc::new(RwLock::new(HostStore::Memory(MemStore::default()))),
        ))
        .unwrap();

        let env = host::get_environment().unwrap();
        let km = EnclaveKeyManager::new(&env.home).unwrap();
        let enclave = Enclave::create(ENCLAVE_FILE, km, env.store.clone()).unwrap();

        match std::env::var(ENV_SETUP_NODES).map(|v| v.to_lowercase()) {
            Ok(v) if v == "false" => run_test(&enclave).unwrap(),
            _ => run_binary_channel_test(&ELCStateVerificationTest { enclave }).unwrap(),
        }
    }

    fn run_test(enclave: &Enclave<store::memory::MemStore>) -> Result<(), anyhow::Error> {
        env_logger::init();
        let rt = Arc::new(TokioRuntime::new()?);
        let rly = config::create_relayer(rt).unwrap();
        verify(rly, enclave)
    }

    fn verify(
        mut rly: Relayer,
        enclave: &Enclave<store::memory::MemStore>,
    ) -> Result<(), anyhow::Error> {
        if cfg!(feature = "sgx-sw") {
            info!("this test is running in SW mode");
        } else {
            info!("this test is running in HW mode");
        }

        let signer = match enclave.generate_enclave_key(GenerateEnclaveKeyInput::default()) {
            Ok(res) => res.pub_key.as_address(),
            Err(e) => {
                bail!("failed to generate an enclave key: {:?}!", e);
            }
        };

        #[cfg(not(feature = "sgx-sw"))]
        {
            let _ =
                match enclave.ias_remote_attestation(ecall_commands::IASRemoteAttestationInput {
                    target_enclave_key: signer,
                    spid: std::env::var("SPID")?.as_bytes().to_vec(),
                    ias_key: std::env::var("IAS_KEY")?.as_bytes().to_vec(),
                }) {
                    Ok(res) => res.report,
                    Err(e) => {
                        bail!("IAS Remote Attestation Failed {:?}!", e);
                    }
                };
        }
        #[cfg(feature = "sgx-sw")]
        {
            use enclave_api::rsa::{pkcs1v15::SigningKey, rand_core::OsRng};
            use enclave_api::sha2::Sha256;
            let _ = match enclave.simulate_remote_attestation(
                ecall_commands::SimulateRemoteAttestationInput {
                    target_enclave_key: signer,
                    advisory_ids: vec![],
                    isv_enclave_quote_status: "OK".to_string(),
                },
                SigningKey::<Sha256>::random(&mut OsRng, 3072)?,
                Default::default(), // TODO set valid certificate
            ) {
                Ok(res) => res.avr,
                Err(e) => {
                    bail!("Simulate Remote Attestation Failed {:?}!", e);
                }
            };
        }

        // XXX use non-latest height here
        let initial_height = rly
            .query_latest_height()?
            .decrement()?
            .decrement()?
            .decrement()?;

        let (client_state, consensus_state) = rly.fetch_state_as_any(initial_height)?;
        info!(
            "initial_height: {:?} client_state: {:?}, consensus_state: {:?}",
            initial_height, client_state, consensus_state
        );

        let res = enclave.init_client(InitClientInput {
            any_client_state: client_state,
            any_consensus_state: consensus_state,
            current_timestamp: Time::now(),
            signer,
        })?;
        assert!(!res.proof.is_proven());
        let client_id = res.client_id;

        info!("generated client id is {}", client_id.as_str().to_string());

        let target_header = rly.create_header(initial_height, initial_height.increment())?;
        let res = enclave.update_client(UpdateClientInput {
            client_id: client_id.clone(),
            any_header: target_header,
            current_timestamp: Time::now(),
            include_state: true,
            signer,
        })?;
        info!("update_client's result is {:?}", res);
        assert!(res.0.is_proven());

        let commitment: UpdateClientCommitment = res.0.commitment().unwrap().try_into()?;
        let height = commitment.new_height;

        info!("current height is {}", height);

        let (port_id, channel_id) = (
            PortId::from_str("transfer")?,
            ChannelId::from_str("channel-0")?,
        );
        let res = rly.query_channel_proof(
            port_id.clone(),
            channel_id.clone(),
            Some(height.try_into().map_err(|e| anyhow!("{:?}", e))?),
        )?;

        info!("expected channel is {:?}", res.0);

        let _ = enclave.verify_membership(VerifyMembershipInput {
            client_id,
            prefix: "ibc".into(),
            path: Path::ChannelEnd(ChannelEndPath(port_id, channel_id)).to_string(),
            value: res.0.encode_vec()?,
            proof: CommitmentProofPair(
                res.2.try_into().map_err(|e| anyhow!("{:?}", e))?,
                merkle_proof_to_bytes(res.1)?,
            ),
            signer,
        })?;

        Ok(())
    }

    fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
        let proof = CommitmentProofBytes::try_from(proof)?;
        Ok(proof.into())
    }
}
