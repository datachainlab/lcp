use dcap_rs::types::collaterals::IntelCollateral;
use dcap_rs::types::quotes::version_3::QuoteV3;
use dcap_rs::utils::quotes::version_3::verify_quote_dcapv3;
use risc0_zkvm::guest::env;

fn main() {
    let (quote, collaterals, current_time): (Vec<u8>, Vec<u8>, u64) = env::read();

    let quote = QuoteV3::from_bytes(&quote).unwrap();
    let collaterals = IntelCollateral::from_bytes(&collaterals).unwrap();

    env::commit_slice(
        verify_quote_dcapv3(&quote, &collaterals, current_time)
            .unwrap()
            .to_bytes()
            .as_slice(),
    );
}
