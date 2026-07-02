use crate::enclave::EnclaveLoader;
use crate::opts::{EnclaveOpts, Opts};
use anyhow::{bail, Context, Result};
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
                let enclave =
                    enclave_loader.load(opts, cmd.enclave.path.as_ref(), cmd.enclave.is_debug())?;
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
                let srv = ElcService::new(
                    opts.get_home(),
                    enclave,
                    speculative_concurrency_limit,
                    enclave_parallelism,
                );
                let runtime_info = srv
                    .enclave_runtime_info()
                    .context("failed to query enclave runtime info")?;
                let tcs_limit = runtime_info.effective_tcs_limit();
                validate_enclave_parallelism(enclave_parallelism, tcs_limit)?;
                srv.prewarm_ecall_pool(enclave_parallelism)
                    .context("failed to prewarm ECALL pool")?;
                if speculative_concurrency_limit > enclave_parallelism {
                    warn!(
                        "max-speculative-concurrency ({}) is greater than max-enclave-concurrency ({}); speculative workers above the enclave limit will block waiting for an EcallPool slot",
                        speculative_concurrency_limit,
                        enclave_parallelism
                    );
                }
                if speculative_concurrency_limit > tcs_limit {
                    warn!(
                        "max-speculative-concurrency ({}) is greater than enclave TCS limit ({}); excess speculative requests will wait for an EcallPool slot",
                        speculative_concurrency_limit,
                        tcs_limit
                    );
                }

                info!(
                    "start service: addr={addr} mrenclave={mrenclave} tcs_limit={} thread_policy={} static_tcs_num={} eremove_tcs_num={} dyn_tcs_num={} tcs_max_num={} edmm_supported={} speculative_concurrency_limit={} enclave_parallelism={}",
                    tcs_limit,
                    runtime_info.thread_policy,
                    runtime_info.static_tcs_num,
                    runtime_info.eremove_tcs_num,
                    runtime_info.dyn_tcs_num,
                    runtime_info.tcs_max_num,
                    runtime_info.edmm_supported,
                    speculative_concurrency_limit,
                    enclave_parallelism
                );
                rt.block_on(async { run_service(srv, addr).await })
            }
        }
    }
}

fn validate_enclave_parallelism(enclave_parallelism: usize, tcs_num: usize) -> Result<()> {
    if enclave_parallelism > tcs_num {
        bail!(
            "max-enclave-concurrency ({}) exceeds enclave TCS limit ({}); reduce --max-enclave-concurrency or rebuild the enclave with a larger TCSNum/TCSMaxNum",
            enclave_parallelism,
            tcs_num
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_enclave_parallelism;

    #[test]
    fn enclave_parallelism_may_equal_tcs_num() {
        validate_enclave_parallelism(8, 8).unwrap();
    }

    #[test]
    fn enclave_parallelism_cannot_exceed_tcs_num() {
        let err = validate_enclave_parallelism(9, 8).unwrap_err();
        assert!(err.to_string().contains("exceeds enclave TCS limit"));
    }
}
