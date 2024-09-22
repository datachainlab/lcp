use std::env;
use std::path::Path;

fn main() {
    let sdk_dir = env::var("SGX_SDK").unwrap_or_else(|_| "/opt/sgxsdk".to_string());
    let sgx_mode = env::var("SGX_MODE").unwrap_or_else(|_| "HW".to_string());

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = Path::new(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();

    println!("cargo:rerun-if-env-changed=SGX_SDK");
    println!("cargo:rerun-if-env-changed=SGX_MODE");
    println!(
        "cargo:rerun-if-changed={}/lib/libEnclave_u.a",
        workspace_root
    );

    println!("cargo:rustc-link-search=native={}/lib", workspace_root);
    println!("cargo:rustc-link-lib=static=Enclave_u");
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
