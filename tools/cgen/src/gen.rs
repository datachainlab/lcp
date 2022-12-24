use crate::types::JSONSerializer;
use anyhow::{anyhow, bail};
use ecall_commands::{
    CommitmentProofPair, IASRemoteAttestationInput, InitClientInput, InitEnclaveInput,
    UpdateClientInput, VerifyMembershipInput,
};
use enclave_api::{Enclave, EnclaveCommandAPI};
use ibc::core::ics04_channel::packet::Sequence;
use ibc::core::ics23_commitment::commitment::CommitmentProofBytes;
use ibc::core::ics23_commitment::merkle::MerkleProof;
use ibc::core::ics24_host::identifier::ClientId;
use ibc::core::ics24_host::path::{ChannelEndsPath, CommitmentsPath};
use ibc::core::ics24_host::Path;
use ibc::Height;
use ibc_test_framework::prelude::*;
use ibc_test_framework::util::random::random_u64_range;
use lcp_types::Time;
use relay_tendermint::Relayer;
use std::str::FromStr;
use std::sync::Arc;
use std::{fs::File, io::Write, path::PathBuf};
use tendermint_proto::Protobuf;
use tokio::runtime::Runtime as TokioRuntime;

pub struct CGenSuite<'e> {
    config: CGenConfig,
    enclave: Enclave<'e, store::memory::MemStore>,
    commands: Vec<Command>,
}

impl<'e> CGenSuite<'e> {
    pub fn new(
        config: CGenConfig,
        enclave: Enclave<'e, store::memory::MemStore>,
        commands: Vec<Command>,
    ) -> Self {
        Self {
            config,
            enclave,
            commands,
        }
    }
}

#[derive(Clone)]
pub struct CGenConfig {
    pub(crate) spid: Vec<u8>,
    pub(crate) ias_key: Vec<u8>,
    pub(crate) out_dir: PathBuf,
}

pub enum Command {
    UpdateClient,
    VerifyChannel,
    VerifyPacket,
}

impl FromStr for Command {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "update_client" => Ok(Command::UpdateClient),
            "verify_channel" => Ok(Command::VerifyChannel),
            "verify_packet" => Ok(Command::VerifyPacket),
            _ => bail!("unknown command: '{}'", s),
        }
    }
}

pub struct CommandFileGenerator<'e, 'f> {
    config: CGenConfig,
    enclave: &'f Enclave<'e, store::memory::MemStore>,
    rly: Relayer,

    channel: (PortId, ChannelId),
    seq: u64,
}

impl<'e, 'f> CommandFileGenerator<'e, 'f> {
    pub fn new(
        config: CGenConfig,
        enclave: &'f Enclave<'e, store::memory::MemStore>,
        rly: Relayer,
        channel: (PortId, ChannelId),
    ) -> Self {
        Self {
            seq: 1,
            config,
            enclave,
            rly,
            channel,
        }
    }

    pub fn gen(&mut self, commands: &[Command]) -> Result<(), anyhow::Error> {
        self.init_enclave_key()?;
        self.seq += 1;
        let (client_id, mut last_height) = self.create_client()?;
        self.seq += 1;

        for cmd in commands.iter() {
            match cmd {
                Command::UpdateClient => {
                    last_height = self.update_client(last_height, client_id.clone())?;
                }
                Command::VerifyChannel => {
                    self.verify_channel(last_height, client_id.clone())?;
                }
                Command::VerifyPacket => {
                    // TODO get sequence from command
                    self.verify_packet(last_height, client_id.clone(), 1u64.into())?;
                }
            };
            self.seq += 1;
        }
        Ok(())
    }

    fn init_enclave_key(&mut self) -> Result<(), anyhow::Error> {
        let _ = match self.enclave.init_enclave_key(InitEnclaveInput::default()) {
            Ok(res) => res,
            Err(e) => {
                bail!("Init Enclave Failed {:?}!", e);
            }
        };

        let res = match self
            .enclave
            .ias_remote_attestation(IASRemoteAttestationInput {
                spid: self.config.spid.clone(),
                ias_key: self.config.ias_key.clone(),
            }) {
            Ok(res) => res.report,
            Err(e) => {
                bail!("IAS Remote Attestation Failed {:?}!", e);
            }
        };

        self.write_to_file("avr", &res)?;
        Ok(())
    }

    fn create_client(&mut self) -> Result<(ClientId, Height), anyhow::Error> {
        // XXX use non-latest height here
        let initial_height = self
            .rly
            .query_latest_height()?
            .decrement()?
            .decrement()?
            .decrement()?;

        let (client_state, consensus_state) = self.rly.fetch_state_as_any(initial_height)?;
        log::info!(
            "initial_height: {:?} client_state: {:?}, consensus_state: {:?}",
            initial_height,
            client_state,
            consensus_state
        );

        let input = InitClientInput {
            any_client_state: client_state.into(),
            any_consensus_state: consensus_state.into(),
            current_timestamp: Time::now(),
        };

        self.write_to_file("init_client_input", &input)?;

        let res = self.enclave.init_client(input).unwrap();
        assert!(!res.proof.is_proven());

        log::info!(
            "generated client id is {}",
            res.client_id.as_str().to_string()
        );

        self.write_to_file("init_client_result", &res)?;

        Ok((res.client_id, initial_height))
    }

    fn update_client(
        &mut self,
        last_height: Height,
        client_id: ClientId,
    ) -> Result<Height, anyhow::Error> {
        let target_header = self.rly.create_header(
            last_height.try_into().map_err(|e| anyhow!("{:?}", e))?,
            last_height
                .increment()
                .try_into()
                .map_err(|e| anyhow!("{:?}", e))?,
        )?;
        let input = UpdateClientInput {
            client_id,
            any_header: target_header.into(),
            current_timestamp: Time::now(),
            include_state: true,
        };

        self.write_to_file("update_client_input", &input)?;

        let res = self.enclave.update_client(input)?;
        log::info!("update_client's result is {:?}", res);
        assert!(res.0.is_proven());

        self.write_to_file("update_client_result", &res.0)?;

        Ok(res.0.commitment().new_height.try_into()?)
    }

    fn verify_channel(
        &mut self,
        last_height: Height,
        client_id: ClientId,
    ) -> Result<(), anyhow::Error> {
        let res = self.rly.query_channel_proof(
            self.channel.0.clone(),
            self.channel.1.clone(),
            Some(last_height.try_into().map_err(|e| anyhow!("{:?}", e))?),
        )?;

        let input = VerifyMembershipInput {
            client_id,
            prefix: "ibc".into(),
            path: Path::ChannelEnds(ChannelEndsPath(
                self.channel.0.clone(),
                self.channel.1.clone(),
            ))
            .to_string(),
            value: res.0.encode_vec().unwrap(),
            proof: CommitmentProofPair(
                res.2.try_into().map_err(|e| anyhow!("{:?}", e))?,
                merkle_proof_to_bytes(res.1)?,
            ),
        };
        self.write_to_file("verify_channel_input", &input)?;
        let res = self.enclave.verify_membership(input)?;
        self.write_to_file("verify_channel_result", &res.0)?;

        Ok(())
    }

    fn verify_packet(
        &mut self,
        last_height: Height,
        client_id: ClientId,
        sequence: Sequence,
    ) -> Result<(), anyhow::Error> {
        let res = self.rly.query_packet_proof(
            self.channel.0.clone(),
            self.channel.1.clone(),
            sequence,
            Some(last_height.try_into().map_err(|e| anyhow!("{:?}", e))?),
        )?;

        let input = VerifyMembershipInput {
            client_id,
            prefix: "ibc".into(),
            path: Path::Commitments(CommitmentsPath {
                port_id: self.channel.0.clone(),
                channel_id: self.channel.1.clone(),
                sequence,
            })
            .to_string(),
            value: res.0.into_vec(),
            proof: CommitmentProofPair(
                res.2.try_into().map_err(|e| anyhow!("{:?}", e))?,
                merkle_proof_to_bytes(res.1)?,
            ),
        };

        self.write_to_file("verify_packet_input", &input)?;
        let res = self.enclave.verify_membership(input)?;
        self.write_to_file("verify_packet_result", &res.0)?;

        Ok(())
    }

    fn write_to_file<S: JSONSerializer>(
        &self,
        name: &str,
        content: &S,
    ) -> Result<(), anyhow::Error> {
        let s = content.to_json_string()?;

        let out_path = self
            .config
            .out_dir
            .join(format!("{:03}-{}", self.seq, name));
        if out_path.exists() {
            bail!(format!("dir '{:?}' already exists", out_path));
        }

        File::create(out_path)?.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl<'e> TestOverrides for CGenSuite<'e> {
    fn modify_relayer_config(&self, config: &mut Config) {
        // disable packet relay
        config.mode.packets.enabled = false;
    }
}

impl<'e> BinaryChannelTest for CGenSuite<'e> {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        _relayer: RelayerDriver,
        chains: ConnectedChains<ChainA, ChainB>,
        channel: ConnectedChannel<ChainA, ChainB>,
    ) -> Result<(), Error> {
        // begin transfer

        let denom_a = chains.node_a.denom();

        let wallet_a = chains.node_a.wallets().user1().cloned();
        let wallet_b = chains.node_b.wallets().user1().cloned();

        let a_to_b_amount = random_u64_range(1000, 5000);

        log::info!(
            "Sending IBC transfer from chain {} to chain {} with amount of {} {}",
            chains.chain_id_a(),
            chains.chain_id_b(),
            a_to_b_amount,
            denom_a
        );

        chains.node_a.chain_driver().ibc_transfer_token(
            &channel.port_a.as_ref(),
            &channel.channel_id_a.as_ref(),
            &wallet_a.as_ref(),
            &wallet_b.address(),
            &denom_a,
            a_to_b_amount,
        )?;

        // end transfer

        let rt = Arc::new(TokioRuntime::new()?);
        let config_a = chains.handle_a().config()?;
        let rly = Relayer::new(config_a, rt).unwrap();
        CommandFileGenerator::new(
            self.config.clone(),
            &self.enclave,
            rly,
            (
                channel.channel.a_side.port_id().clone(),
                channel.channel.a_side.channel_id().unwrap().clone(),
            ),
        )
        .gen(&self.commands)
        .map_err(|e| Error::assertion(e.to_string()))
    }
}

fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
    let proof = CommitmentProofBytes::try_from(proof)?;
    Ok(proof.into())
}
