pub mod errors;

use std::rc::Rc;
use risc0_zkvm::{default_executor, default_prover, ExecutorEnv, LocalProver, Prover, ProverOpts, VerifierContext};
use risc0_zkvm::BonsaiProver;

pub fn local_prover() -> LocalProver {
    LocalProver::new("local")
}

pub fn bonsai_prover() -> BonsaiProver {
    BonsaiProver::new("bonsai")
}
