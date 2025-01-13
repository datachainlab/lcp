use dcap_rs::types::collaterals::IntelCollateral;
use risc0_zkvm::guest::env;
use dcap_rs::utils::quotes::version_3::verify_quote_dcapv3;
use dcap_rs::types::quotes::version_3::QuoteV3;
use dcap_rs::types::DCAPVerifierCommit;

fn main() {
    let (quote, collaterals, current_time): (Vec<u8>, Vec<u8>, u64) = env::read();

    let quote = QuoteV3::from_bytes(&quote);
    let collaterals = IntelCollateral::from_bytes(&collaterals);

    let output = verify_quote_dcapv3(&quote, &collaterals, current_time);
    let commit = DCAPVerifierCommit::new(current_time, output, collaterals.sgx_intel_root_ca_der.unwrap().as_slice());
    env::commit_slice(commit.to_bytes().as_slice());
}
