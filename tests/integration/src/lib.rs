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
    use anyhow::bail;
    use attestation_report::QEType;
    use commitments::UpdateStateProxyMessage;
    use crypto::Address;
    use ecall_commands::{
        AggregateMessagesInput, CommitmentProofPair, GenerateEnclaveKeyInput, InitClientInput,
        UpdateClientInput, VerifyMembershipInput,
    };
    use enclave_api::{Enclave, EnclaveCommandAPI, EnclaveInfo};
    use host_environment::Environment;
    use ibc::{
        core::{
            ics23_commitment::{commitment::CommitmentProofBytes, merkle::MerkleProof},
            ics24_host::{
                identifier::{ChannelId, PortId},
                path::ChannelEndPath,
                Path,
            },
        },
        Height as IBCHeight,
    };
    use ibc_test_framework::prelude::{
        run_binary_channel_test, BinaryChannelTest, ChainHandle, Config, ConnectedChains,
        ConnectedChannel, Error, RelayerDriver, TestConfig, TestOverrides,
    };
    use keymanager::EnclaveKeyManager;
    use lcp_proto::protobuf::Protobuf;
    use lcp_types::{ClientId, Height};
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

    fn test_remote_attestation(
        enclave: &Enclave<store::memory::MemStore>,
    ) -> Result<(), anyhow::Error> {
        if cfg!(feature = "sgx-sw") {
            info!("this test is running in SW mode");
        } else {
            info!("this test is running in HW mode");
        }

        let (qe_target_info, _) = remote_attestation::init_quote(QEType::QE)?;
        let operator = Address::from_hex_string("0x396e1ccc2f11cd6d2114c2449dad7751357e413e")?;
        let op_ek_addr = match enclave.generate_enclave_key(
            GenerateEnclaveKeyInput {
                operator: Some(operator),
                target_info: qe_target_info,
            },
            QEType::QE,
        ) {
            Ok(res) => res.pub_key.as_address(),
            Err(e) => {
                bail!("failed to generate an enclave key: {:?}!", e);
            }
        };
        let ek_addr = match enclave.generate_enclave_key(
            GenerateEnclaveKeyInput {
                operator: None,
                target_info: qe_target_info,
            },
            QEType::QE,
        ) {
            Ok(res) => res.pub_key.as_address(),
            Err(e) => {
                bail!("failed to generate an enclave key: {:?}!", e);
            }
        };

        #[cfg(not(feature = "sgx-sw"))]
        {
            use remote_attestation::ias::run_ias_ra;
            let res = match run_ias_ra(
                enclave.get_key_manager(),
                op_ek_addr,
                remote_attestation::IASMode::Production,
                std::env::var("SPID")?,
                std::env::var("IAS_KEY")?,
            ) {
                Ok(res) => res,
                Err(e) => {
                    bail!("IAS Remote Attestation Failed {:?}!", e);
                }
            };
            let report_data = res.get_avr()?.parse_quote()?.report_data();
            assert_eq!(report_data.enclave_key(), op_ek_addr);
            assert_eq!(report_data.operator(), operator);

            let res = match run_ias_ra(
                enclave.get_key_manager(),
                ek_addr,
                remote_attestation::IASMode::Production,
                std::env::var("SPID")?,
                std::env::var("IAS_KEY")?,
            ) {
                Ok(res) => res,
                Err(e) => {
                    panic!("IAS Remote Attestation Failed {:?}!", e);
                }
            };
            let report_data = res.get_avr()?.parse_quote()?.report_data();
            assert_eq!(report_data.enclave_key(), ek_addr);
            assert!(report_data.operator().is_zero());
        }
        #[cfg(feature = "sgx-sw")]
        {
            use remote_attestation::ias_simulation::run_ias_ra_simulation;
            use remote_attestation::rsa::{pkcs1v15::SigningKey, rand_core::OsRng};
            use remote_attestation::sha2::Sha256;

            let res = match run_ias_ra_simulation(
                enclave.get_key_manager(),
                op_ek_addr,
                vec![],
                "OK".to_string(),
                SigningKey::<Sha256>::random(&mut OsRng, 3072)?,
                Default::default(), // TODO set valid certificate
            ) {
                Ok(res) => res.get_avr()?,
                Err(e) => {
                    bail!("Remote Attestation Simulation Failed {:?}!", e);
                }
            };
            let report_data = res.parse_quote()?.report_data();
            assert_eq!(report_data.enclave_key(), op_ek_addr);
            assert_eq!(report_data.operator(), operator);

            let res = match run_ias_ra_simulation(
                enclave.get_key_manager(),
                ek_addr,
                vec![],
                "OK".to_string(),
                SigningKey::<Sha256>::random(&mut OsRng, 3072)?,
                Default::default(), // TODO set valid certificate
            ) {
                Ok(res) => res.get_avr()?,
                Err(e) => {
                    bail!("Remote Attestation Simulation Failed {:?}!", e);
                }
            };
            let report_data = res.parse_quote()?.report_data();
            assert_eq!(report_data.enclave_key(), ek_addr);
            assert!(report_data.operator().is_zero());
        }

        {
            use remote_attestation::dcap_simulation::DCAPRASimulationOpts;
            use remote_attestation::dcap_simulation::{
                DCAP_SIM_ROOT_CA_PEM, DCAP_SIM_ROOT_KEY_PKCS8,
            };
            use remote_attestation::zkdcap::run_zkdcap_ra_simulation;
            use remote_attestation::zkvm::prover::Risc0ProverMode;
            use zkdcap_risc0::DCAP_QUOTE_VERIFIER_ELF;

            let target_info = remote_attestation::init_quote(QEType::QE3SIM)?.0;
            let zkdcap_ek_addr = match enclave.generate_enclave_key(
                GenerateEnclaveKeyInput {
                    operator: Some(operator),
                    target_info,
                },
                QEType::QE3SIM,
            ) {
                Ok(res) => res.pub_key.as_address(),
                Err(e) => {
                    bail!("failed to generate an enclave key: {:?}!", e);
                }
            };

            let res = run_zkdcap_ra_simulation(
                enclave.get_key_manager(),
                zkdcap_ek_addr,
                Risc0ProverMode::Dev,
                DCAP_QUOTE_VERIFIER_ELF,
                false,
                DCAPRASimulationOpts::new(
                    DCAP_SIM_ROOT_CA_PEM.as_bytes(),
                    DCAP_SIM_ROOT_KEY_PKCS8.as_bytes(),
                )?,
            );
            assert!(
                res.is_ok(),
                "zkDCAP Remote Attestation Simulation Failed {:?}",
                res
            );
        }
        #[cfg(not(feature = "sgx-sw"))]
        {
            use remote_attestation::dcap_pcs::client::PCSClient;
            use remote_attestation::zkdcap::run_zkdcap_ra;
            use remote_attestation::zkvm::prover::Risc0ProverMode;
            use zkdcap_risc0::DCAP_QUOTE_VERIFIER_ELF;

            let target_info = remote_attestation::init_quote(QEType::QE3)?.0;
            let zkdcap_ek_addr = match enclave.generate_enclave_key(
                GenerateEnclaveKeyInput {
                    operator: Some(operator),
                    target_info,
                },
                QEType::QE3,
            ) {
                Ok(res) => res.pub_key.as_address(),
                Err(e) => {
                    bail!("failed to generate an enclave key: {:?}!", e);
                }
            };

            let res = run_zkdcap_ra(
                enclave.get_key_manager(),
                zkdcap_ek_addr,
                Risc0ProverMode::Dev,
                DCAP_QUOTE_VERIFIER_ELF,
                false,
                PCSClient::new(
                    "https://api.trustedservices.intel.com/",
                    "https://certificates.trustedservices.intel.com/",
                    false,
                ),
            );
            assert!(res.is_ok(), "zkDCAP Remote Attestation Failed {:?}", res);
        }

        Ok(())
    }

    #[test]
    fn test_lcp() {
        let tmp_dir = TempDir::new().unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();
        host::set_environment(Environment::new(
            home.into(),
            Arc::new(RwLock::new(HostStore::Memory(MemStore::default()))),
        ))
        .unwrap();

        let env = host::get_environment().unwrap();
        let km = EnclaveKeyManager::new(&env.home).unwrap();
        let enclave = Enclave::create(ENCLAVE_FILE, true, km, env.store.clone()).unwrap();

        test_remote_attestation(&enclave).unwrap();

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
        let operator = Address::from_hex_string("0x396e1ccc2f11cd6d2114c2449dad7751357e413e")?;
        let (target_info, _) = remote_attestation::init_quote(QEType::QE)?;
        let signer = match enclave.generate_enclave_key(
            GenerateEnclaveKeyInput {
                operator: Some(operator),
                target_info,
            },
            QEType::QE,
        ) {
            Ok(res) => res.pub_key.as_address(),
            Err(e) => {
                bail!("failed to generate an enclave key: {:?}!", e);
            }
        };

        let (client_id, last_height) = {
            // XXX use non-latest height here
            let initial_height = rly.query_latest_height()?.decrement()?.decrement()?;

            let (client_state, consensus_state) = rly.fetch_state_as_any(initial_height)?;
            info!(
                "initial_height: {:?} client_state: {:?}, consensus_state: {:?}",
                initial_height, client_state, consensus_state
            );

            let client_id = "07-tendermint-0".to_string();
            let res = enclave.init_client(InitClientInput {
                client_id: client_id.clone(),
                any_client_state: client_state,
                any_consensus_state: consensus_state,
                signer,
            })?;
            assert!(!res.proof.is_proven());

            (ClientId::from_str(&client_id)?, initial_height)
        };
        info!("generated client: id={} height={}", client_id, last_height);

        let last_height = {
            let post_height = last_height.increment();
            let target_header = rly.create_header(last_height, post_height)?;
            let res = enclave.update_client(UpdateClientInput {
                client_id: client_id.clone(),
                any_header: target_header,
                include_state: true,
                signer,
            })?;
            info!("update_client's result is {:?}", res);
            assert!(res.0.is_proven());

            let msg: UpdateStateProxyMessage = res.0.message().unwrap().try_into()?;
            assert!(msg.prev_height == Some(Height::from(last_height)));
            assert!(msg.post_height == Height::from(post_height));
            assert!(msg.emitted_states.len() == 1);
            post_height
        };
        info!("current last_height is {}", last_height);

        {
            let (port_id, channel_id) = (
                PortId::from_str("transfer")?,
                ChannelId::from_str("channel-0")?,
            );
            let res =
                rly.query_channel_proof(port_id.clone(), channel_id.clone(), Some(last_height))?;

            info!("expected channel is {:?}", res.0);

            let _ = enclave.verify_membership(VerifyMembershipInput {
                client_id: client_id.clone(),
                prefix: "ibc".into(),
                path: Path::ChannelEnd(ChannelEndPath(port_id, channel_id)).to_string(),
                value: res.0.encode_vec()?,
                proof: CommitmentProofPair(res.2.into(), merkle_proof_to_bytes(res.1)?),
                signer,
            })?;
        }

        let last_height = {
            let mut lh = last_height;
            let mut proofs = vec![];
            for _ in 0..10 {
                let target_height = wait_block_advance(&mut rly)?;
                let target_header = rly.create_header(lh, target_height)?;
                let res = enclave.update_client(UpdateClientInput {
                    client_id: client_id.clone(),
                    any_header: target_header,
                    include_state: false,
                    signer,
                })?;
                info!("update_client's result is {:?}", res);
                lh = target_height;
                proofs.push(res.0);
            }
            let messages = proofs
                .iter()
                .map(|p| p.message().map(|m| m.to_bytes()))
                .collect::<Result<_, _>>()?;
            let signatures = proofs.into_iter().map(|p| p.signature).collect();

            let res = enclave.aggregate_messages(AggregateMessagesInput {
                messages,
                signatures,
                signer,
            })?;
            let msg: UpdateStateProxyMessage = res.0.message().unwrap().try_into()?;
            assert!(msg.prev_height == Some(Height::from(last_height)));
            assert!(msg.post_height == Height::from(lh));
            assert!(msg.emitted_states.is_empty());
            lh
        };
        info!("current last_height is {}", last_height);

        Ok(())
    }

    fn wait_block_advance(rly: &mut Relayer) -> Result<IBCHeight, anyhow::Error> {
        let mut height = rly.query_latest_height()?;
        loop {
            let next_height = rly.query_latest_height()?;
            if next_height > height {
                height = next_height;
                break;
            }
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
        Ok(height)
    }

    fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
        let proof = CommitmentProofBytes::try_from(proof)?;
        Ok(proof.into())
    }
}
