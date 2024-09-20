use std::env;

fn main() {
    let sdk_dir = env::var("SGX_SDK").unwrap_or_else(|_| "/opt/sgxsdk".to_string());
    let sgx_mode = env::var("SGX_MODE").unwrap_or_else(|_| "HW".to_string());

    println!("cargo:rerun-if-env-changed=SGX_SDK");
    println!("cargo:rerun-if-env-changed=SGX_MODE");
    println!("cargo:rustc-link-search=native={}/lib64", sdk_dir);
    println!("cargo:rustc-link-lib=dylib=sgx_dcap_ql");
    match sgx_mode.as_ref() {
        "SW" => {
            println!("cargo:rustc-link-lib=dylib=sgx_uae_service_sim");
        }
        "HW" => {
            println!("cargo:rustc-link-lib=dylib=sgx_uae_service");
        }
        _ => {
            println!("cargo:rustc-link-lib=dylib=sgx_uae_service");
        }
    }
}
