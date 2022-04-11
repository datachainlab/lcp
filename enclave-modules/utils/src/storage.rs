use crate::results::UnwrapOrSgxErrorUnexpected;

use sgx_types::*;
use std::format;
use std::io::{Read, Write};
use std::untrusted::fs::File;
use std::vec::Vec;

pub fn write_to_untrusted(bytes: &[u8], filepath: &str) -> SgxResult<()> {
    let mut f = File::create(filepath)
        .sgx_error_with_log(&format!("Creating file '{}' failed", filepath))?;
    f.write_all(bytes)
        .sgx_error_with_log("[Enclave] Writing File failed!")
}

pub fn read_from_untrusted(filepath: &str) -> SgxResult<Vec<u8>> {
    let mut f =
        File::open(filepath).sgx_error_with_log(&format!("Opening file '{}' failed", filepath))?;
    let mut buf = Vec::new();
    let _ = f
        .read_to_end(&mut buf)
        .sgx_error_with_log("[Enclave] Writing File failed!")?;
    Ok(buf)
}
