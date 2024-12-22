use alloy_primitives::U256;
use alloy_sol_types::SolValue;
use dcap_rs::types::collaterals::IntelCollateral;
use risc0_zkvm::guest::env;
use dcap_rs::utils::quotes::version_3::verify_quote_dcapv3;
use dcap_rs::types::quotes::version_3::QuoteV3;

fn main() {
    let (quote, collaterals, current_time): (Vec<u8>, Vec<u8>, u64) = env::read();

    let quote = QuoteV3::from_bytes(&quote);
    let collaterals = IntelCollateral::from_bytes(&collaterals);

    let output = verify_quote_dcapv3(&quote, &collaterals, current_time);
    env::commit_slice(output.to_bytes().as_slice());
}
