pub mod errors;

use risc0_zkvm::BonsaiProver;
use risc0_zkvm::{
    default_executor, default_prover, ExecutorEnv, LocalProver, Prover, ProverOpts, VerifierContext,
};
use std::rc::Rc;

pub fn local_prover() -> LocalProver {
    LocalProver::new("local")
}

pub fn bonsai_prover() -> BonsaiProver {
    BonsaiProver::new("bonsai")
}
