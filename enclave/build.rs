fn main() {
    sgx_build::SgxBuilder::new()
        .build_enclave("Enclave.edl")
        .expect("Failed to build SGX enclave");
}
