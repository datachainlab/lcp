use crate::errors::Error;
use attestation_report::SignedAttestationVerificationReport;
use base64::{engine::general_purpose::STANDARD as Base64Std, Engine};
use log::*;
use rand::RngCore;
use rustls::RootCertStore;
use sgx_types::{
    sgx_calc_quote_size, sgx_epid_group_id_t, sgx_get_quote, sgx_init_quote, sgx_quote_nonce_t,
    sgx_quote_sign_type_t, sgx_quote_t, sgx_report_t, sgx_spid_t, sgx_status_t, sgx_target_info_t,
};
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::ptr;
use std::str;
use std::sync::Arc;

pub const IAS_HOSTNAME: &str = "api.trustedservices.intel.com";
pub const IAS_HTTPS_PORT: u16 = 443;
pub const SGX_QUOTE_SIGN_TYPE: sgx_quote_sign_type_t =
    sgx_quote_sign_type_t::SGX_UNLINKABLE_SIGNATURE;

#[derive(Debug, Clone, Copy)]
pub enum IASMode {
    Development,
    Production,
}

impl Display for IASMode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IASMode::Development => write!(f, "Development"),
            IASMode::Production => write!(f, "Production"),
        }
    }
}

impl IASMode {
    pub const fn get_sigrl_suffix(&self) -> &'static str {
        match self {
            IASMode::Development => "/sgx/dev/attestation/v4/sigrl/",
            IASMode::Production => "/sgx/attestation/v4/sigrl/",
        }
    }

    pub const fn get_report_suffix(&self) -> &'static str {
        match self {
            IASMode::Development => "/sgx/dev/attestation/v4/report",
            IASMode::Production => "/sgx/attestation/v4/report",
        }
    }
}

pub fn init_quote() -> Result<(sgx_target_info_t, sgx_epid_group_id_t), Error> {
    let mut target_info = sgx_target_info_t::default();
    let mut epid_group_id = sgx_epid_group_id_t::default();
    match unsafe { sgx_init_quote(&mut target_info, &mut epid_group_id) } {
        sgx_status_t::SGX_SUCCESS => Ok((target_info, epid_group_id)),
        s => Err(Error::sgx_error(s, "failed to sgx_init_quote".into())),
    }
}

pub(crate) fn get_quote(
    sigrl: Vec<u8>,
    report: sgx_report_t,
    quote_type: sgx_quote_sign_type_t,
    spid: sgx_spid_t,
) -> Result<(Vec<u8>, sgx_report_t), Error> {
    let mut quote_nonce = sgx_quote_nonce_t { rand: [0; 16] };
    rand::rngs::OsRng.fill_bytes(&mut quote_nonce.rand);

    let (p_sigrl, sigrl_size) = if sigrl.is_empty() {
        (ptr::null(), 0)
    } else {
        (sigrl.as_ptr(), sigrl.len() as u32)
    };

    let (quote, qe_report) = {
        let mut quote_size: u32 = 0;
        let ret = unsafe { sgx_calc_quote_size(p_sigrl, sigrl_size, &mut quote_size as *mut u32) };
        if ret != sgx_status_t::SGX_SUCCESS {
            return Err(Error::sgx_error(
                ret,
                "failed to sgx_calc_quote_size".into(),
            ));
        }
        info!("quote size = {}", quote_size);

        let mut qe_report = sgx_report_t::default();
        let quote: Vec<u8> = vec![0; quote_size as usize];
        let p_quote = quote.as_ptr();
        let ret = unsafe {
            sgx_get_quote(
                &report,
                quote_type,
                &spid,
                &quote_nonce,
                p_sigrl,
                sigrl_size,
                &mut qe_report,
                p_quote as *mut sgx_quote_t,
                quote_size,
            )
        };
        if ret != sgx_status_t::SGX_SUCCESS {
            return Err(Error::sgx_error(ret, "failed to sgx_get_quote".into()));
        }
        (quote, qe_report)
    };

    // Check qe_report to defend against replay attack
    // The purpose of p_qe_report is for the ISV enclave to confirm the QUOTE
    // it received is not modified by the untrusted SW stack, and not a replay.
    // The implementation in QE is to generate a REPORT targeting the ISV
    // enclave (target info from p_report) , with the lower 32Bytes in
    // report.data = SHA256(p_nonce||p_quote). The ISV enclave can verify the
    // p_qe_report and report.data to confirm the QUOTE has not be modified and
    // is not a replay. It is optional.

    let mut rhs_vec: Vec<u8> = quote_nonce.rand.to_vec();
    rhs_vec.extend(&quote);
    let mut hasher = Sha256::new();
    hasher.update(&rhs_vec);
    let h = hasher.finalize();
    let rhs_hash = h.as_slice();
    let lhs_hash = &qe_report.body.report_data.d[..32];

    trace!("Report rhs hash = {}", hex::encode(rhs_hash));
    trace!("Report lhs hash = {}", hex::encode(lhs_hash));

    if rhs_hash != lhs_hash {
        return Err(Error::unexpected_quote(
            format!("Quote is tampered!: {:?} != {:?}", rhs_hash, lhs_hash).to_string(),
        ));
    }

    Ok((quote, qe_report))
}

pub(crate) fn get_sigrl_from_intel(
    mode: IASMode,
    gid: [u8; 4],
    ias_key: &str,
) -> Result<Vec<u8>, Error> {
    info!("using IAS mode: {}", mode);
    let config = make_ias_client_config();
    let req = format!("GET {}{:08x} HTTP/1.1\r\nHOST: {}\r\nOcp-Apim-Subscription-Key: {}\r\nConnection: Close\r\n\r\n",
        mode.get_sigrl_suffix(),
        u32::from_le_bytes(gid),
        IAS_HOSTNAME,
        ias_key);

    trace!("get_sigrl_from_intel: {}", req);
    let mut sess = rustls::ClientConnection::new(
        Arc::new(config),
        IAS_HOSTNAME
            .try_into()
            .map_err(|_| Error::invalid_ias_server_name())?,
    )
    .map_err(Error::rustls)?;
    let mut sock = TcpStream::connect(lookup_ipv4(IAS_HOSTNAME, IAS_HTTPS_PORT)?)
        .map_err(|e| Error::io_error(e, "failed to connect to IAS server".to_string()))?;
    let mut tls = rustls::Stream::new(&mut sess, &mut sock);

    let _result = tls.write(req.as_bytes());
    let mut plaintext = Vec::new();

    info!("write complete");

    match tls.read_to_end(&mut plaintext) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            // IAS server may not send `close_notify` message, so we can ignore this error.
            // ref. https://docs.rs/rustls/0.23.17/rustls/manual/_03_howto/index.html#unexpected-eof
            info!("probably IAS server did not send close_notify message");
        }
        Err(e) => {
            return Err(Error::io_error(
                e,
                "failed to read response from IAS server".to_string(),
            ));
        }
    }
    info!("read_to_end complete");
    parse_response_sigrl(&plaintext)
}

pub(crate) fn get_report_from_intel(
    mode: IASMode,
    quote: Vec<u8>,
    ias_key: &str,
) -> Result<SignedAttestationVerificationReport, Error> {
    info!("using IAS mode: {}", mode);
    let config = make_ias_client_config();
    let encoded_quote = Base64Std.encode(&quote[..]);
    let encoded_json = format!("{{\"isvEnclaveQuote\":\"{}\"}}\r\n", encoded_quote);

    let req = format!("POST {} HTTP/1.1\r\nHOST: {}\r\nOcp-Apim-Subscription-Key:{}\r\nContent-Length:{}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                      mode.get_report_suffix(),
                      IAS_HOSTNAME,
                      ias_key,
                      encoded_json.len(),
                      encoded_json);

    trace!("{}", req);
    let mut sess = rustls::ClientConnection::new(
        Arc::new(config),
        IAS_HOSTNAME
            .try_into()
            .map_err(|_| Error::invalid_ias_server_name())?,
    )
    .map_err(Error::rustls)?;
    let mut sock = TcpStream::connect(lookup_ipv4(IAS_HOSTNAME, IAS_HTTPS_PORT)?)
        .map_err(|e| Error::io_error(e, "Failed to connect to IAS server".to_string()))?;
    let mut tls = rustls::Stream::new(&mut sess, &mut sock);

    let _result = tls.write(req.as_bytes());
    let mut plaintext = Vec::new();

    info!("write complete");

    match tls.read_to_end(&mut plaintext) {
        Ok(_) => (),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            // IAS server may not send `close_notify` message, so we can ignore this error.
            // ref. https://docs.rs/rustls/0.23.17/rustls/manual/_03_howto/index.html#unexpected-eof
            info!("probably IAS server did not send close_notify message");
        }
        Err(e) => {
            return Err(Error::io_error(
                e,
                "failed to read response from IAS server".to_string(),
            ));
        }
    }
    info!("read_to_end complete");

    parse_response_attn_report(&plaintext)
}

pub fn validate_qe_report(
    target_info: &sgx_target_info_t,
    qe_report: &sgx_report_t,
) -> Result<(), Error> {
    // Check if the qe_report is produced on the same platform
    if target_info.mr_enclave.m != qe_report.body.mr_enclave.m
        || target_info.attributes.flags != qe_report.body.attributes.flags
        || target_info.attributes.xfrm != qe_report.body.attributes.xfrm
    {
        return Err(Error::unexpected_report(
            "qe_report does not match current target_info!".to_string(),
        ));
    }
    Ok(())
}

fn parse_response_attn_report(resp: &[u8]) -> Result<SignedAttestationVerificationReport, Error> {
    trace!("parse_response_attn_report");
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut respp = httparse::Response::new(&mut headers);
    let result = respp.parse(resp);
    trace!("parse result {:?}", result);
    match respp.code {
        Some(200) => info!("OK Operation Successful"),
        Some(401) => return Err(Error::unexpected_ias_report_response("unauthorized Failed to authenticate or authorize request".to_string())),
        Some(404) => return Err(Error::unexpected_ias_report_response("not Found GID does not refer to a valid EPID group ID".to_string())),
        Some(500) => return Err(Error::unexpected_ias_report_response("internal error occurred".to_string())),
        Some(503) => return Err(Error::unexpected_ias_report_response("service is currently not able to process the request (due to a temporary overloading or maintenance). This is a temporary state – the same request can be repeated after some time.".to_string())),
        _ => return Err(Error::unexpected_ias_report_response(format!("unknown error occured: {:?}", respp.code))),
    }

    let mut len_num: u32 = 0;
    let mut sig = String::new();
    let mut cert = String::new();
    let mut attn_report = String::new();
    for i in 0..respp.headers.len() {
        let h = respp.headers[i];
        match h.name {
            "Content-Length" => {
                let len_str = str::from_utf8(h.value).map_err(|e| {
                    Error::invalid_utf8_bytes(h.value.to_vec(), e, h.name.to_string())
                })?;
                len_num = len_str.parse::<u32>().map_err(|e| {
                    Error::invalid_u32_string(len_str.to_string(), e, h.name.to_string())
                })?;
                trace!("content length = {}", len_num);
            }
            "X-IASReport-Signature" => {
                sig = str::from_utf8(h.value)
                    .map_err(|e| {
                        Error::invalid_utf8_bytes(h.value.to_vec(), e, h.name.to_string())
                    })?
                    .to_string()
            }
            "X-IASReport-Signing-Certificate" => {
                cert = str::from_utf8(h.value)
                    .map_err(|e| {
                        Error::invalid_utf8_bytes(h.value.to_vec(), e, h.name.to_string())
                    })?
                    .to_string()
            }
            _ => (),
        }
    }

    // Remove %0A from cert, and only obtain the signing cert
    cert = percent_decode(cert.replace("%0A", ""))?;

    let v: Vec<&str> = cert.split("-----").collect();
    if v.len() < 3 {
        return Err(Error::unexpected_ias_report_certificate_response(
            "invalid signing certificate".to_string(),
        ));
    }
    let sig_cert = v[2].to_string();

    if len_num != 0 {
        let status = result.map_err(Error::http_parse_error)?;
        let header_len = if status.is_complete() {
            status.unwrap()
        } else {
            return Err(Error::http_parse_partial_status());
        };
        let resp_body = &resp[header_len..];
        attn_report = str::from_utf8(resp_body)
            .map_err(|e| {
                Error::invalid_utf8_bytes(resp_body.to_vec(), e, "Attestation Report".to_string())
            })?
            .to_string();
        info!("Attestation report: {}", attn_report);
    }

    let signature = Base64Std
        .decode(&sig)
        .map_err(|e| Error::base64_decode(e, "Signature".to_string()))?;
    let signing_cert = Base64Std
        .decode(&sig_cert)
        .map_err(|e| Error::base64_decode(e, "Signing Certificate".to_string()))?;
    Ok(SignedAttestationVerificationReport {
        avr: attn_report,
        signature,
        signing_cert,
    })
}

fn parse_response_sigrl(resp: &[u8]) -> Result<Vec<u8>, Error> {
    trace!("parse_response_sigrl");
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut respp = httparse::Response::new(&mut headers);
    let result = respp.parse(resp);
    trace!("parse result {:?}", result);
    trace!("parse response{:?}", respp);

    match respp.code {
        Some(200) => info!("OK Operation Successful"),
        Some(401) => return Err(Error::unexpected_sigrl_response("unauthorized Failed to authenticate or authorize request".to_string())),
        Some(404) => return Err(Error::unexpected_sigrl_response("not Found GID does not refer to a valid EPID group ID".to_string())),
        Some(500) => return Err(Error::unexpected_sigrl_response("internal error occurred".to_string())),
        Some(503) => return Err(Error::unexpected_sigrl_response("service is currently not able to process the request (due to a temporary overloading or maintenance). This is a temporary state – the same request can be repeated after some time.".to_string())),
        _ => return Err(Error::unexpected_sigrl_response(format!("unknown error occured: {:?}", respp.code))),
    }

    let mut len_num: u32 = 0;
    for i in 0..respp.headers.len() {
        let h = respp.headers[i];
        if h.name == "content-length" {
            let len_str = str::from_utf8(h.value).map_err(|e| {
                Error::invalid_utf8_bytes(h.value.to_vec(), e, "Content-Length".to_string())
            })?;
            len_num = len_str.parse::<u32>().map_err(|e| {
                Error::invalid_u32_string(len_str.to_string(), e, "Content-Length".to_string())
            })?;
            trace!("content length = {}", len_num);
        }
    }

    if len_num != 0 {
        let status = result.map_err(Error::http_parse_error)?;
        let header_len = if status.is_complete() {
            status.unwrap()
        } else {
            return Err(Error::http_parse_partial_status());
        };
        let resp_body = &resp[header_len..];
        trace!("Base64-encoded SigRL: {:?}", resp_body);

        Ok(Base64Std
            .decode(resp_body)
            .map_err(|e| Error::base64_decode(e, "SigRL".to_string()))?)
    } else {
        Ok(Vec::new())
    }
}

fn make_ias_client_config() -> rustls::ClientConfig {
    let builder = rustls::ClientConfig::builder();
    let root_store = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.into(),
    };
    builder
        .with_root_certificates(root_store)
        .with_no_client_auth()
}

fn percent_decode(orig: String) -> Result<String, Error> {
    let v: Vec<&str> = orig.split("%").collect();
    let mut ret = String::new();
    ret.push_str(v[0]);
    if v.len() > 1 {
        for s in v[1..].iter() {
            ret.push(
                u8::from_str_radix(&s[0..2], 16).map_err(|e| {
                    Error::invalid_percent_decode(format!("failed to decode: {}", e))
                })? as char,
            );
            ret.push_str(&s[2..]);
        }
    }
    Ok(ret)
}

fn lookup_ipv4(host: &str, port: u16) -> Result<SocketAddr, Error> {
    let addrs = (host, port)
        .to_socket_addrs()
        .map_err(|_| Error::cannot_lookup_address(host.to_string(), port))?;
    for addr in addrs {
        if let SocketAddr::V4(_) = addr {
            return Ok(addr);
        }
    }
    Err(Error::cannot_lookup_address(host.to_string(), port))
}

pub(crate) fn decode_spid(spid_str: &str) -> Result<sgx_spid_t, Error> {
    let spid_str = spid_str.trim();
    if spid_str.len() != 32 {
        return Err(Error::invalid_spid(format!(
            "invalid length: {}",
            spid_str.len()
        )));
    }
    let decoded_vec = match hex::decode(spid_str) {
        Ok(v) => v,
        Err(_) => {
            return Err(Error::invalid_spid("failed to decode".to_string()));
        }
    };
    let mut spid = sgx_spid_t::default();
    // the length of `decoded_vec` is 16 because each byte is represented by 2 characters
    spid.id.copy_from_slice(&decoded_vec);
    Ok(spid)
}
