use attestation_report::EndorsedAttestationReport;
use enclave_api::{Enclave, EnclaveAPI};
use enclave_commands::{CommandResult, CommitmentProofPair, LightClientResult, UpdateClientResult};
use ibc::core::{
    ics02_client::{client_def::ClientDef, height::Height},
    ics23_commitment::commitment::CommitmentProofBytes,
    ics24_host::identifier::{ChannelId, PortId},
};
use ibc_proto::ibc::core::commitment::v1::MerkleProof;
use log::*;
use settings::ENDORSED_ATTESTATION_PATH;
use sgx_types::*;
use sgx_urts::SgxEnclave;
use std::{str::FromStr, sync::Arc};
use tokio::runtime::Runtime as TokioRuntime;

mod ocalls;
mod relayer;

static ENCLAVE_FILE: &'static str = "enclave.signed.so";

fn init_enclave() -> SgxResult<SgxEnclave> {
    let mut launch_token: sgx_launch_token_t = [0; 1024];
    let mut launch_token_updated: i32 = 0;
    // call sgx_create_enclave to initialize an enclave instance
    // Debug Support: set 2nd parameter to 1
    let debug = 1;
    let mut misc_attr = sgx_misc_attribute_t {
        secs_attr: sgx_attributes_t { flags: 0, xfrm: 0 },
        misc_select: 0,
    };
    SgxEnclave::create(
        ENCLAVE_FILE,
        debug,
        &mut launch_token,
        &mut launch_token_updated,
        &mut misc_attr,
    )
}

fn main() -> Result<(), anyhow::Error> {
    env_logger::init();
    let rt = Arc::new(TokioRuntime::new()?);

    let spid = std::env::var("SPID")?;
    let ias_key = std::env::var("IAS_KEY")?;

    let enclave = match init_enclave() {
        Ok(r) => {
            info!("[+] Init Enclave Successful {}!", r.geteid());
            r
        }
        Err(x) => {
            panic!("[-] Init Enclave Failed {}!", x.as_str());
        }
    };

    let enclave = Enclave::new(enclave);

    if let Err(e) = enclave.init_enclave_key(spid.as_bytes(), ias_key.as_bytes()) {
        panic!("[-] ECALL Enclave Failed {:?}!", e);
    } else {
        info!("[+] remote attestation success...");
    }

    let report = EndorsedAttestationReport::read_from_file(&ENDORSED_ATTESTATION_PATH).unwrap();
    let quote = attestation_report::parse_quote_from_report(&report.report).unwrap();
    info!("report={:?}", quote.raw.report_body.report_data.d);

    // register the key into onchain

    let mut rly = relayer::create_relayer(rt, "ibc0")?;

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

    let proof = if let CommandResult::LightClient(LightClientResult::InitClient(res)) = enclave
        .init_client("07-tendermint", client_state, consensus_state)
        .unwrap()
    {
        res.0
    } else {
        panic!("unexpected result type")
    };
    let commitment = proof.commitment();
    let client_id = commitment.client_id;

    info!("generated client id is {}", client_id.as_str().to_string());

    let target_header =
        rly.create_header(commitment.new_height, commitment.new_height.increment())?;
    let res = enclave.update_client(client_id.clone(), target_header.into())?;

    info!("update_client's result is {:?}", res);

    let height = match res {
        CommandResult::LightClient(LightClientResult::UpdateClient(UpdateClientResult(res))) => {
            res.commitment().new_height
        }
        _ => unreachable!(),
    };

    info!("current height is {}", height);

    let (port_id, channel_id) = (
        PortId::from_str("cross")?,
        ChannelId::from_str("channel-0")?,
    );
    let res = rly.proven_channel(&port_id, &channel_id, Some(height))?;

    info!("expected channel is {:?}", res.0);

    let res = enclave.verify_channel(
        client_id.clone(),
        res.0,
        "ibc".into(),
        port_id,
        channel_id,
        CommitmentProofPair(res.2, merkle_proof_to_bytes(res.1)?),
    )?;

    let cs = lcp_ibc_client::client_state::ClientState::default();
    let client = lcp_ibc_client::client_def::LCPClient {};

    // TODO implement the verification testing

    enclave.destroy();

    Ok(())
}

fn merkle_proof_to_bytes(proof: MerkleProof) -> Result<Vec<u8>, anyhow::Error> {
    let proof = CommitmentProofBytes::try_from(proof)?;
    Ok(proof.into())
}
