use std::env;
use std::path::Path;

fn main() {
    let sdk_dir = env::var("SGX_SDK").unwrap_or_else(|_| "/opt/sgxsdk".to_string());
    let sgx_mode = env::var("SGX_MODE").unwrap_or_else(|_| "HW".to_string());

    env::set_var("CARGO_TARGET_DIR", env::var("OUT_DIR").unwrap());
    let edl = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../enclave/Enclave.edl");
    sgx_build::SgxBuilder::new()
        .build_app(edl)
        .expect("Failed to build SGX app");

    println!("cargo:rustc-link-search=native={}/lib64", sdk_dir);

    match sgx_mode.as_ref() {
        "SW" => {
            println!("cargo:rustc-link-lib=dylib=sgx_urts_sim");
        }
        "HW" => {
            println!("cargo:rustc-link-lib=dylib=sgx_urts");
        }
        _ => {
            println!("cargo:rustc-link-lib=dylib=sgx_urts");
        }
    }
}
