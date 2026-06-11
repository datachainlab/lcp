use crate::enclave::EnclaveLoader;
use crate::opts::{EnclaveOpts, Opts};
use anyhow::Result;
use clap::Parser;
use enclave_api::{Enclave, EnclaveInfo, EnclaveProtoAPI, SpeculativeEnclaveCommandAPI};
use host::store::transaction::{CommitStore, TxAccessor};
use log::*;
use service::{run_service, ElcService};
use std::sync::Arc;
use tokio::runtime::Builder;

// `service` subcommand
#[derive(Debug, Parser)]
pub enum ServiceCmd {
    #[clap(about = "Start the App service")]
    Start(Start),
}

#[derive(Clone, Debug, Parser, PartialEq)]
pub struct Start {
    /// Options for enclave
    #[clap(flatten)]
    pub enclave: EnclaveOpts,
    /// Address of the App service
    #[clap(
        long = "address",
        default_value = "[::1]:50051",
        help = "Address of the App service"
    )]
    pub address: String,
    /// Worker thread number the tokio `Runtime` will use.
    /// This does not control enclave ECALL/TCS concurrency.
    #[clap(
        long = "threads",
        help = "Worker thread number the tokio `Runtime` will use"
    )]
    pub threads: Option<usize>,
    /// Size of the dedicated ECALL worker pool. All enclave ECALL execution
    /// flows through this pool — both serial gRPC handlers and speculative
    /// scheduler workers — so this value is the single source of truth for
    /// concurrent ECALL count and cumulative TCS bindings under
    /// `TCSPolicy=BIND`. Set this to a value at most equal to the loaded
    /// enclave's `TCSNum`; leaving at least one TCS for the SDK runtime
    /// (i.e. `--max-enclave-concurrency = TCSNum - 1`) is the conservative
    /// default.
    #[clap(
        long = "max-enclave-concurrency",
        default_value_t = 4,
        help = "Size of the dedicated ECALL worker pool"
    )]
    pub max_enclave_concurrency: usize,
    /// Maximum concurrent speculative update-client requests.
    /// Prefer a value less than or equal to --max-enclave-concurrency; excess
    /// speculative workers will wait on the enclave ECALL gate.
    #[clap(
        long = "max-speculative-concurrency",
        default_value_t = 1,
        help = "Maximum concurrent speculative update-client requests"
    )]
    pub max_speculative_concurrency: usize,
}

impl ServiceCmd {
    pub fn run<S, L>(&self, opts: &Opts, enclave_loader: L) -> Result<()>
    where
        S: CommitStore + TxAccessor + 'static,
        Enclave<S>: EnclaveProtoAPI<S> + SpeculativeEnclaveCommandAPI<S>,
        L: EnclaveLoader<S>,
    {
        match self {
            Self::Start(cmd) => {
                let addr = cmd.address.parse()?;
                let enclave_parallelism = cmd.max_enclave_concurrency.max(1);
                let enclave = enclave_loader.load(
                    opts,
                    cmd.enclave.path.as_ref(),
                    cmd.enclave.is_debug(),
                )?;
                let metadata = enclave.metadata()?;
                let mrenclave = metadata.mrenclave().to_hex_string();
                let mut rb = Builder::new_multi_thread();
                let rb = if let Some(threads) = cmd.threads {
                    rb.worker_threads(threads)
                } else {
                    &mut rb
                };
                let rt = Arc::new(rb.enable_all().build()?);
                let speculative_concurrency_limit = cmd.max_speculative_concurrency.max(1);
                if speculative_concurrency_limit > enclave_parallelism {
                    warn!(
                        "max-speculative-concurrency ({}) is greater than max-enclave-concurrency ({}); speculative workers above the enclave limit will block waiting for an EcallPool slot",
                        speculative_concurrency_limit,
                        enclave_parallelism
                    );
                }
                let srv = ElcService::new(
                    opts.get_home(),
                    enclave,
                    speculative_concurrency_limit,
                    enclave_parallelism,
                );

                info!(
                    "start service: addr={addr} mrenclave={mrenclave} speculative_concurrency_limit={} enclave_parallelism={}",
                    speculative_concurrency_limit,
                    enclave_parallelism
                );
                rt.block_on(async { run_service(srv, addr).await })
            }
        }
    }
}
