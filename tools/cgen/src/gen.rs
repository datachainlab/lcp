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
use ibc::core::ics24_host::path::{ChannelEndPath, CommitmentPath, ConnectionPath};
use ibc::core::ics24_host::Path;
use ibc::Height;
use ibc_proto::protobuf::Protobuf;
use ibc_test_framework::prelude::*;
use ibc_test_framework::util::random::random_u64_range;
use lcp_types::{ClientId, Time};
use relay_tendermint::types::{to_ibc_channel_id, to_ibc_connection_id, to_ibc_port_id};
use relay_tendermint::Relayer;
use std::str::FromStr;
use std::sync::Arc;
use std::{fs::File, io::Write, path::PathBuf};
use tokio::runtime::Runtime as TokioRuntime;

pub struct CGenSuite {
    config: CGenConfig,
    enclave: Enclave<store::memory::MemStore>,
    commands: Vec<Command>,
}

impl CGenSuite {
    pub fn new(
        config: CGenConfig,
        enclave: Enclave<store::memory::MemStore>,
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
    VerifyConnection,
    VerifyChannel,
    VerifyPacket,
    WaitBlocks(u64),
}

impl FromStr for Command {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split(':').collect();
        match parts[0] {
            "update_client" => Ok(Command::UpdateClient),
            "verify_connection" => Ok(Command::VerifyConnection),
            "verify_channel" => Ok(Command::VerifyChannel),
            "verify_packet" => Ok(Command::VerifyPacket),
            "wait_blocks" => {
                if parts.len() != 2 {
                    bail!("`wait` requires one argument");
                }
                Ok(Command::WaitBlocks(u64::from_str(parts[1])?))
            }
            _ => bail!("unknown command: '{}'", s),
        }
    }
}

pub struct CommandFileGenerator<'e, ChainA: ChainHandle, ChainB: ChainHandle> {
    config: CGenConfig,
    enclave: &'e Enclave<store::memory::MemStore>,
    rly: Relayer,

    channel: ConnectedChannel<ChainA, ChainB>,
    command_sequence: u64,

    client_latest_height: Option<Height>, // latest height of client state
    chain_latest_provable_height: Height, // latest provable height of chainA
}

impl<'e, ChainA: ChainHandle, ChainB: ChainHandle> CommandFileGenerator<'e, ChainA, ChainB> {
    pub fn new(
        config: CGenConfig,
        enclave: &'e Enclave<store::memory::MemStore>,
        rly: Relayer,
        channel: ConnectedChannel<ChainA, ChainB>,
    ) -> Self {
        let chain_latest_provable_height = rly.query_latest_height().unwrap().decrement().unwrap();
        Self {
            config,
            enclave,
            rly,
            channel,
            command_sequence: 1,
            client_latest_height: None,
            chain_latest_provable_height,
        }
    }

    pub fn gen(&mut self, commands: &[Command], wait_blocks: u64) -> Result<(), anyhow::Error> {
        if wait_blocks > 0 {
            self.wait_blocks(wait_blocks)?;
        }
        self.init_enclave_key()?;
        self.command_sequence += 1;
        let client_id = self.create_client()?;
        self.command_sequence += 1;

        for cmd in commands.iter() {
            assert!(self.command_sequence < 1000);
            match cmd {
                Command::UpdateClient => self.update_client(client_id.clone())?,
                Command::VerifyConnection => self.verify_connection(client_id.clone())?,
                Command::VerifyChannel => self.verify_channel(client_id.clone())?,
                // TODO get sequence from command
                Command::VerifyPacket => self.verify_packet(client_id.clone(), 1u64.into())?,
                Command::WaitBlocks(n) => self.wait_blocks(*n)?,
            };
            self.command_sequence += 1;
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

    fn create_client(&mut self) -> Result<ClientId, anyhow::Error> {
        let (client_state, consensus_state) = self
            .rly
            .fetch_state_as_any(self.chain_latest_provable_height)?;
        log::info!(
            "initial_height: {:?} client_state: {:?}, consensus_state: {:?}",
            self.chain_latest_provable_height,
            client_state,
            consensus_state
        );

        let input = InitClientInput {
            any_client_state: client_state,
            any_consensus_state: consensus_state,
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

        self.client_latest_height = Some(self.chain_latest_provable_height);

        Ok(res.client_id)
    }

    fn update_client(&mut self, client_id: ClientId) -> Result<(), anyhow::Error> {
        assert!(
            self.chain_latest_provable_height > self.client_latest_height.unwrap(),
            "To update the client, you need to advance block's height with `wait_blocks`"
        );
        let target_header = self.rly.create_header(
            self.client_latest_height.unwrap(),
            self.chain_latest_provable_height,
        )?;
        let input = UpdateClientInput {
            client_id,
            any_header: target_header,
            current_timestamp: Time::now(),
            include_state: true,
        };

        self.write_to_file("update_client_input", &input)?;

        let res = self.enclave.update_client(input)?;
        log::info!("update_client's result is {:?}", res);
        assert!(res.0.is_proven());

        self.write_to_file("update_client_result", &res.0)?;

        assert!(self.chain_latest_provable_height == res.0.commitment().new_height.try_into()?);
        self.client_latest_height = Some(self.chain_latest_provable_height);
        Ok(())
    }

    fn verify_connection(&mut self, client_id: ClientId) -> Result<(), anyhow::Error> {
        let res = self.rly.query_connection_proof(
            to_ibc_connection_id(
                self.channel
                    .connection
                    .connection
                    .a_connection_id()
                    .unwrap()
                    .clone(),
            ),
            self.client_latest_height,
        )?;

        let input = VerifyMembershipInput {
            client_id,
            prefix: "ibc".into(),
            path: Path::Connection(ConnectionPath(to_ibc_connection_id(
                self.channel
                    .connection
                    .connection
                    .a_connection_id()
                    .unwrap()
                    .clone(),
            )))
            .to_string(),
            value: res.0.encode_vec().unwrap(),
            proof: CommitmentProofPair(
                res.2.try_into().map_err(|e| anyhow!("{:?}", e))?,
                merkle_proof_to_bytes(res.1)?,
            ),
        };
        self.write_to_file("verify_connection_input", &input)?;
        let res = self.enclave.verify_membership(input)?;
        self.write_to_file("verify_connection_result", &res.0)?;

        Ok(())
    }

    fn verify_channel(&mut self, client_id: ClientId) -> Result<(), anyhow::Error> {
        let res = self.rly.query_channel_proof(
            to_ibc_port_id(self.channel.channel.a_side.port_id().clone()),
            to_ibc_channel_id(self.channel.channel.a_side.channel_id().unwrap().clone()),
            self.client_latest_height,
        )?;

        let input = VerifyMembershipInput {
            client_id,
            prefix: "ibc".into(),
            path: Path::ChannelEnd(ChannelEndPath(
                to_ibc_port_id(self.channel.channel.a_side.port_id().clone()),
                to_ibc_channel_id(self.channel.channel.a_side.channel_id().unwrap().clone()),
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
        client_id: ClientId,
        sequence: Sequence,
    ) -> Result<(), anyhow::Error> {
        let res = self.rly.query_packet_proof(
            to_ibc_port_id(self.channel.channel.a_side.port_id().clone()),
            to_ibc_channel_id(self.channel.channel.a_side.channel_id().unwrap().clone()),
            sequence,
            self.client_latest_height,
        )?;

        let input = VerifyMembershipInput {
            client_id,
            prefix: "ibc".into(),
            path: Path::Commitment(CommitmentPath {
                port_id: to_ibc_port_id(self.channel.channel.a_side.port_id().clone()),
                channel_id: to_ibc_channel_id(
                    self.channel.channel.a_side.channel_id().unwrap().clone(),
                ),
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

    fn wait_blocks(&mut self, n: u64) -> Result<(), anyhow::Error> {
        let target = self.chain_latest_provable_height.add(n);
        loop {
            let h = self.rly.query_latest_height()?.decrement()?;
            info!(
                "wait_blocks: found new height: height={} target={}",
                h, target
            );
            if h > target {
                self.chain_latest_provable_height = target;
                return Ok(());
            }
        }
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
            .join(format!("{:03}-{}", self.command_sequence, name));
        if out_path.exists() {
            bail!(format!("dir '{:?}' already exists", out_path));
        }

        File::create(out_path)?.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl TestOverrides for CGenSuite {
    fn modify_relayer_config(&self, config: &mut Config) {
        // disable packet relay
        config.mode.packets.enabled = false;
    }
}

impl BinaryChannelTest for CGenSuite {
    fn run<ChainA: ChainHandle, ChainB: ChainHandle>(
        &self,
        _config: &TestConfig,
        _relayer: RelayerDriver,
        chains: ConnectedChains<ChainA, ChainB>,
        channel: ConnectedChannel<ChainA, ChainB>,
    ) -> Result<(), Error> {
        // Begin: IBC transfer

        let denom_a = chains.node_a.denom();
        let wallet_a = chains.node_a.wallets().user1().cloned();
        let wallet_b = chains.node_b.wallets().user1().cloned();
        let balance_a = chains
            .node_a
            .chain_driver()
            .query_balance(&wallet_a.address(), &denom_a)?;

        let a_to_b_amount = random_u64_range(1000, 5000);

        chains.node_a.chain_driver().ibc_transfer_token(
            &channel.port_a.as_ref(),
            &channel.channel_id_a.as_ref(),
            &wallet_a.as_ref(),
            &wallet_b.address(),
            &denom_a.with_amount(a_to_b_amount).as_ref(),
        )?;

        chains.node_a.chain_driver().assert_eventual_wallet_amount(
            &wallet_a.address(),
            &denom_a
                .with_amount(balance_a.amount().checked_sub(a_to_b_amount).unwrap())
                .as_ref(),
        )?;

        log::info!(
            "Sending IBC transfer from chain {} to chain {} with amount of {} {}",
            chains.chain_id_a(),
            chains.chain_id_b(),
            a_to_b_amount,
            denom_a
        );

        // End: IBC transfer

        let rt = Arc::new(TokioRuntime::new()?);
        let config_a = chains.handle_a().config()?;
        let rly = Relayer::new(config_a, rt).unwrap();
        CommandFileGenerator::new(self.config.clone(), &self.enclave, rly, channel)
            .gen(&self.commands, 1)
            .map_err(|e| Error::assertion(e.to_string()))
    }
}

fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
    let proof = CommitmentProofBytes::try_from(proof)?;
    Ok(proof.into())
}
