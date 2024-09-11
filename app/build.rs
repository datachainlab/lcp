use git2::{DescribeOptions, Repository};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdk_dir = env::var("SGX_SDK").unwrap_or_else(|_| "/opt/sgxsdk".to_string());
    let sgx_mode = env::var("SGX_MODE").unwrap_or_else(|_| "HW".to_string());
    let mut opts = DescribeOptions::new();
    opts.describe_tags().show_commit_oid_as_fallback(true);
    let version = Repository::discover(".")?.describe(&opts)?.format(None)?;
    println!("cargo:rustc-env=LCP_VERSION={}", version);
    println!("cargo:rustc-link-search=native=./lib");
    println!("cargo:rustc-link-lib=static=Enclave_u");
    println!("cargo:rustc-link-search=native={}/lib64", sdk_dir);

    match sgx_mode.as_ref() {
        "SW" => {
            println!("cargo:rustc-link-lib=dylib=sgx_urts_sim");
            println!("cargo:rustc-link-lib=dylib=sgx_uae_service_sim");
        }
        "HW" => {
            println!("cargo:rustc-link-lib=dylib=sgx_urts");
            println!("cargo:rustc-link-lib=dylib=sgx_uae_service");
        }
        _ => {
            println!("cargo:rustc-link-lib=dylib=sgx_urts");
            println!("cargo:rustc-link-lib=dylib=sgx_uae_service");
        }
    }
    Ok(())
}
