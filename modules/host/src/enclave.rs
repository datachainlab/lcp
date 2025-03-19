use lcp_types::EnclaveMetadata;
use sgx_types::{metadata::metadata_t, *};
use sgx_urts::SgxEnclave;
use std::{ffi::CString, mem::MaybeUninit, path::PathBuf};

/// Create an enclave with the given path and debug flag
///
/// # Arguments
/// - `path`: path to the enclave binary
/// - `debug`: debug flag
pub fn create_enclave(path: impl Into<PathBuf>, debug: bool) -> SgxResult<SgxEnclave> {
    let mut launch_token: sgx_launch_token_t = [0; 1024];
    let mut launch_token_updated: i32 = 0;
    let mut misc_attr = sgx_misc_attribute_t {
        secs_attr: sgx_attributes_t { flags: 0, xfrm: 0 },
        misc_select: 0,
    };
    SgxEnclave::create(
        path.into(),
        debug.into(),
        &mut launch_token,
        &mut launch_token_updated,
        &mut misc_attr,
    )
}

/// Get the metadata of an enclave from the given path
///
/// # Arguments
/// - `path`: path to the enclave binary
pub fn sgx_get_metadata(path: impl Into<PathBuf>) -> SgxResult<EnclaveMetadata> {
    let path = path.into();
    let enclave_path = CString::new(path.as_os_str().to_str().unwrap()).unwrap();
    let (metadata, status) = unsafe {
        let mut metadata: metadata_t = MaybeUninit::zeroed().assume_init();
        let status = sgx_types::sgx_get_metadata(enclave_path.as_ptr(), &mut metadata);
        (metadata, status)
    };
    if status == sgx_status_t::SGX_SUCCESS {
        Ok(metadata.into())
    } else {
        Err(status)
    }
}
