
<!-- cargo-rdme start -->

# dcap-qvl

This crate implements the quote verification logic for DCAP (Data Center Attestation Primitives) in pure Rust. It supports both SGX (Software Guard Extensions) and TDX (Trust Domain Extensions) quotes.

# Features
- Verify SGX and TDX quotes
- Get collateral from PCCS
- Extract information from quotes

# Usage
Add the following dependency to your `Cargo.toml` file to use this crate:
```toml
[dependencies]
dcap-qvl = "0.1.0"
```

# Example: Get Collateral from PCCS_URL and Verify Quote

To get collateral from a PCCS_URL and verify a quote, you can use the following example code:
```rust
use dcap_qvl::collateral::get_collateral;
use dcap_qvl::verify::verify;

#[tokio::main]
async fn main() {
    // Get PCCS_URL from environment variable. The URL is like "https://localhost:8081/sgx/certification/v4/".
    let pccs_url = std::env::var("PCCS_URL").expect("PCCS_URL is not set");
    let quote = std::fs::read("tdx_quote").expect("tdx_quote is not found");
    let collateral = get_collateral(&pccs_url, &quote, std::time::Duration::from_secs(10)).await.expect("failed to get collateral");
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let tcb = verify(&quote, &collateral, now).expect("failed to verify quote");
    println!("{:?}", tcb);
}
```

<!-- cargo-rdme end -->

# License

This crate is licensed under the MIT license. See the LICENSE file for details.
