#[cfg(test)]
mod relayer;
#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::{anyhow, bail};
    use ecall_commands::{
        CommitmentProofPair, IASRemoteAttestationInput, InitClientInput, InitEnclaveInput,
        UpdateClientInput, VerifyMembershipInput,
    };
    use enclave_api::{Enclave, EnclaveCommandAPI};
    use host_environment::Environment;
    use ibc::core::{
        ics23_commitment::{commitment::CommitmentProofBytes, merkle::MerkleProof},
        ics24_host::{
            identifier::{ChannelId, PortId},
            path::ChannelEndsPath,
            Path,
        },
    };
    use ibc_test_framework::prelude::{
        run_binary_channel_test, BinaryChannelTest, ChainHandle, Config, ConnectedChains,
        ConnectedChannel, Error, RelayerDriver, TestConfig, TestOverrides,
    };
    use lcp_types::Time;
    use log::*;
    use relay_tendermint::Relayer;
    use std::str::FromStr;
    use std::sync::{Arc, RwLock};
    use store::{host::HostStore, memory::MemStore};
    use tempdir::TempDir;
    use tendermint_proto::Protobuf;
    use tokio::runtime::Runtime as TokioRuntime;

    static ENCLAVE_FILE: &'static str = "../../bin/enclave.signed.so";
    static ENV_SETUP_NODES: &'static str = "SETUP_NODES";

    struct ELCStateVerificationTest<'e> {
        enclave: Enclave<'e, store::memory::MemStore>,
    }

    impl<'e> TestOverrides for ELCStateVerificationTest<'e> {
        fn modify_relayer_config(&self, _config: &mut Config) {}
    }

    impl<'e> BinaryChannelTest for ELCStateVerificationTest<'e> {
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
        let tmp_dir = TempDir::new("lcp").unwrap();
        let home = tmp_dir.path().to_str().unwrap().to_string();
        host::set_environment(Environment::new(
            home.clone().into(),
            Arc::new(RwLock::new(HostStore::Memory(MemStore::default()))),
        ))
        .unwrap();

        let enclave = match host::load_enclave(ENCLAVE_FILE) {
            Ok(r) => {
                info!("Init Enclave Successful {}!", r.geteid());
                r
            }
            Err(x) => {
                panic!("Init Enclave Failed {}!", x.as_str());
            }
        };
        let enclave = Enclave::new(enclave, host::get_environment().unwrap());

        match std::env::var(ENV_SETUP_NODES).map(|v| v.to_lowercase()) {
            Ok(v) if v == "false" => run_test(&enclave).unwrap(),
            _ => run_binary_channel_test(&ELCStateVerificationTest { enclave }).unwrap(),
        }
    }

    fn run_test<'e>(enclave: &Enclave<'e, store::memory::MemStore>) -> Result<(), anyhow::Error> {
        env_logger::init();
        let rt = Arc::new(TokioRuntime::new()?);
        let rly = relayer::create_relayer(rt).unwrap();
        verify(rly, enclave)
    }

    fn verify<'e>(
        mut rly: Relayer,
        enclave: &Enclave<'e, store::memory::MemStore>,
    ) -> Result<(), anyhow::Error> {
        let simulate = std::env::var("SGX_MODE").map_or(false, |m| m == "SW");
        if simulate {
            info!("this test is running in SW mode");
        } else {
            info!("this test is running in HW mode");
        }

        let _ = match enclave.init_enclave_key(InitEnclaveInput::default()) {
            Ok(res) => res,
            Err(e) => {
                bail!("Init Enclave Failed {:?}!", e);
            }
        };

        let simulate = std::env::var("SGX_MODE").map_or(false, |m| m == "SW");
        if !simulate {
            let _ = match enclave.ias_remote_attestation(IASRemoteAttestationInput {
                spid: std::env::var("SPID").unwrap().as_bytes().to_vec(),
                ias_key: std::env::var("IAS_KEY").unwrap().as_bytes().to_vec(),
            }) {
                Ok(res) => res.report,
                Err(e) => {
                    bail!("IAS Remote Attestation Failed {:?}!", e);
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

        let res = enclave
            .init_client(InitClientInput {
                any_client_state: client_state.into(),
                any_consensus_state: consensus_state.into(),
                current_timestamp: Time::now(),
            })
            .unwrap();
        assert!(!res.proof.is_proven());
        let client_id = res.client_id;

        info!("generated client id is {}", client_id.as_str().to_string());

        let target_header = rly.create_header(
            initial_height.try_into().map_err(|e| anyhow!("{:?}", e))?,
            initial_height
                .increment()
                .try_into()
                .map_err(|e| anyhow!("{:?}", e))?,
        )?;
        let res = enclave.update_client(UpdateClientInput {
            client_id: client_id.clone(),
            any_header: target_header.into(),
            current_timestamp: Time::now(),
            include_state: true,
        })?;
        info!("update_client's result is {:?}", res);
        assert!(res.0.is_proven());

        let height = res.0.commitment().new_height;

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
            path: Path::ChannelEnds(ChannelEndsPath(port_id, channel_id)).to_string(),
            value: res.0.encode_vec().unwrap(),
            proof: CommitmentProofPair(
                res.2.try_into().map_err(|e| anyhow!("{:?}", e))?,
                merkle_proof_to_bytes(res.1)?,
            ),
        })?;

        Ok(())
    }

    fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
        let proof = CommitmentProofBytes::try_from(proof)?;
        Ok(proof.into())
    }
}
