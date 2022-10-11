use log::*;
use std::net::{SocketAddr, TcpStream};
use std::os::unix::io::IntoRawFd;
use std::ptr;

use crate::errors::{Error, Result};
use ocall_commands::{
    GetIASSocketResult, GetQuoteInput, GetQuoteResult, GetReportAttestationStatusInput,
    GetReportAttestationStatusResult, InitQuoteResult, RemoteAttestationCommand,
    RemoteAttestationResult,
};
use sgx_types::*;

pub fn dispatch(command: RemoteAttestationCommand) -> Result<RemoteAttestationResult> {
    use RemoteAttestationCommand::*;

    let res = match command {
        InitQuote => RemoteAttestationResult::InitQuote(init_quote()?),
        GetIASSocket => RemoteAttestationResult::GetIASSocket(get_ias_socket()?),
        GetQuote(input) => RemoteAttestationResult::GetQuote(get_quote(input)?),
        GetReportAttestationStatus(input) => RemoteAttestationResult::GetReportAttestationStatus(
            get_report_attestation_status(input)?,
        ),
    };
    Ok(res)
}

fn init_quote() -> Result<InitQuoteResult> {
    let mut target_info = sgx_target_info_t::default();
    let mut epid_group_id = sgx_epid_group_id_t::default();
    match unsafe { sgx_init_quote(&mut target_info, &mut epid_group_id) } {
        sgx_status_t::SGX_SUCCESS => {}
        s => return Err(Error::sgx_error(s, "failed to sgx_init_quote".into())),
    }
    Ok(InitQuoteResult {
        target_info,
        epid_group_id,
    })
}

fn get_ias_socket() -> Result<GetIASSocketResult> {
    let port = 443;
    let hostname = "api.trustedservices.intel.com";
    let addr = lookup_ipv4(hostname, port);
    let sock = TcpStream::connect(&addr).expect("[-] Connect tls server failed!");

    Ok(GetIASSocketResult {
        fd: sock.into_raw_fd(),
    })
}

fn get_quote(input: GetQuoteInput) -> Result<GetQuoteResult> {
    let mut quote_size: u32 = 0;

    let (p_sigrl, sigrl_len) = if input.sigrl.is_empty() {
        (ptr::null(), 0)
    } else {
        (input.sigrl.as_ptr(), input.sigrl.len() as u32)
    };

    let ret = unsafe { sgx_calc_quote_size(p_sigrl, sigrl_len, &mut quote_size as *mut u32) };

    if ret != sgx_status_t::SGX_SUCCESS {
        return Err(Error::sgx_error(
            ret,
            "failed to sgx_calc_quote_size".into(),
        ));
    }

    info!("quote size = {}", quote_size);
    let mut qe_report = sgx_report_t::default();
    let quote = [0u8; 2048];
    let p_quote = quote.as_ptr();

    let ret = unsafe {
        sgx_get_quote(
            &input.report,
            input.quote_type,
            &input.spid,
            &input.nonce,
            p_sigrl,
            sigrl_len,
            &mut qe_report,
            p_quote as *mut sgx_quote_t,
            quote_size,
        )
    };

    if ret != sgx_status_t::SGX_SUCCESS {
        return Err(Error::sgx_error(ret, "failed to sgx_get_quote".into()));
    }

    Ok(GetQuoteResult {
        qe_report,
        quote: quote[..quote_size as usize].to_vec(),
    })
}

fn get_report_attestation_status(
    input: GetReportAttestationStatusInput,
) -> Result<GetReportAttestationStatusResult> {
    let mut update_info = sgx_update_info_bit_t::default();
    let ret = unsafe {
        sgx_report_attestation_status(
            &input.platform_blob,
            input.enclave_trusted,
            &mut update_info,
        )
    };
    // TODO validate `ret`
    Ok(GetReportAttestationStatusResult { ret, update_info })
}

fn lookup_ipv4(host: &str, port: u16) -> SocketAddr {
    use std::net::ToSocketAddrs;

    let addrs = (host, port).to_socket_addrs().unwrap();
    for addr in addrs {
        if let SocketAddr::V4(_) = addr {
            return addr;
        }
    }

    unreachable!("Cannot lookup address");
}
