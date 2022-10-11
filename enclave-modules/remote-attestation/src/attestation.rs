use crate::errors::Error;
use crate::prelude::*;
use alloc::str;
use attestation_report::EndorsedAttestationVerificationReport;
use crypto::sgx::rand::fill_bytes;
use host_api::remote_attestation::{get_ias_socket, get_quote, init_quote};
use itertools::Itertools;
use log::*;
use ocall_commands::{GetIASSocketResult, GetQuoteInput, GetQuoteResult, InitQuoteResult};
use settings::{SigningMethod, SIGNING_METHOD};
use sgx_tcrypto::rsgx_sha256_slice;
use sgx_tse::{rsgx_create_report, rsgx_verify_report};
use sgx_tstd::{
    io::{Read, Write},
    net::TcpStream,
    sync::Arc,
};
use sgx_types::{c_int, sgx_spid_t};
use sgx_types::{sgx_quote_nonce_t, sgx_quote_sign_type_t, sgx_report_data_t};

const REPORT_DATA_SIZE: usize = 32;

pub const DEV_HOSTNAME: &str = "api.trustedservices.intel.com";

#[cfg(feature = "production")]
pub const SIGRL_SUFFIX: &str = "/sgx/attestation/v4/sigrl/";
#[cfg(feature = "production")]
pub const REPORT_SUFFIX: &str = "/sgx/attestation/v4/report";

#[cfg(all(not(feature = "production")))]
pub const SIGRL_SUFFIX: &str = "/sgx/dev/attestation/v4/sigrl/";
#[cfg(all(not(feature = "production")))]
pub const REPORT_SUFFIX: &str = "/sgx/dev/attestation/v4/report";

//input: pub_k: &sgx_ec256_public_t, todo: make this the pubkey of the node
#[allow(const_err)]
pub fn create_attestation_report(
    report_data: sgx_report_data_t,
    sign_type: sgx_quote_sign_type_t,
    spid: sgx_spid_t,
    api_hex_str_bytes: &[u8],
) -> Result<EndorsedAttestationVerificationReport, Error> {
    // Workflow:
    // (1) ocall to get the target_info structure and epid_group_id
    // (1.5) get sigrl
    // (2) call sgx_create_report with target_info+data, produce an sgx_report_t
    // (3) ocall to sgx_get_quote to generate (*mut sgx-quote_t, uint32_t)

    // (1) get target_info + epid_group_id

    let InitQuoteResult {
        target_info,
        epid_group_id,
    } = init_quote().map_err(Error::host_api)?;

    trace!("EPID group = {:?}", epid_group_id);

    let eg_num = as_u32_le(&epid_group_id);

    // (1.5) get sigrl
    let GetIASSocketResult { fd } = get_ias_socket().map_err(Error::host_api)?;

    trace!("Got ias_sock successfully = {}", fd);

    // Now sigrl_vec is the revocation list, a vec<u8>
    let sigrl_vec: Vec<u8> = get_sigrl_from_intel(fd, eg_num, api_hex_str_bytes);

    // (2) Generate the report
    // Fill secp256k1 public key into report_data
    // this is given as a parameter

    let report = match rsgx_create_report(&target_info, &report_data) {
        Ok(r) => {
            match SIGNING_METHOD {
                SigningMethod::MRENCLAVE => {
                    trace!(
                        "Report creation => success. Using MR_SIGNER: {:?}",
                        r.body.mr_signer.m
                    );
                }
                SigningMethod::MRSIGNER => {
                    trace!(
                        "Report creation => success. Got MR_ENCLAVE {:?}",
                        r.body.mr_signer.m
                    );
                }
                SigningMethod::NONE => {
                    trace!("Report creation => success. Not using any verification");
                }
            }
            r
        }
        Err(e) => {
            return Err(Error::sgx_error(e, "Report creation => failed".to_string()));
        }
    };

    let mut quote_nonce = sgx_quote_nonce_t { rand: [0; 16] };
    fill_bytes(&mut quote_nonce.rand)
        .map_err(|e| Error::sgx_error(e, "failed to fill_bytes".to_string()))?;
    trace!("Nonce generated successfully");

    // (3) Generate the quote
    // Args:
    //       1. sigrl: ptr + len
    //       2. report: ptr 432bytes
    //       3. linkable: u32, unlinkable=0, linkable=1
    //       4. spid: sgx_spid_t ptr 16bytes
    //       5. sgx_quote_nonce_t ptr 16bytes
    //       6. p_sig_rl + sigrl size ( same to sigrl)
    //       7. [out]p_qe_report need further check
    //       8. [out]p_quote
    //       9. quote_size

    let GetQuoteResult { qe_report, quote } = get_quote(GetQuoteInput {
        sigrl: sigrl_vec,
        report,
        quote_type: sign_type,
        spid,
        nonce: quote_nonce,
    })
    .map_err(Error::host_api)?;

    // Added 09-28-2018
    // Perform a check on qe_report to verify if the qe_report is valid
    match rsgx_verify_report(&qe_report) {
        Ok(()) => trace!("rsgx_verify_report passed!"),
        Err(e) => {
            return Err(Error::sgx_error(e, "rsgx_verify_report failed".to_string()));
        }
    }

    // Check if the qe_report is produced on the same platform
    if target_info.mr_enclave.m != qe_report.body.mr_enclave.m
        || target_info.attributes.flags != qe_report.body.attributes.flags
        || target_info.attributes.xfrm != qe_report.body.attributes.xfrm
    {
        return Err(Error::unexpected_report(
            "qe_report does not match current target_info!".to_string(),
        ));
    }

    trace!("QE report check passed");

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
    let rhs_hash = rsgx_sha256_slice(&rhs_vec[..]).unwrap();
    let lhs_hash = &qe_report.body.report_data.d[..REPORT_DATA_SIZE];

    trace!("Report rhs hash = {:02X}", rhs_hash.iter().format(""));
    trace!("Report lhs hash = {:02X}", lhs_hash.iter().format(""));

    if rhs_hash != lhs_hash {
        return Err(Error::unexpected_quote(
            format!("Quote is tampered!: {:?} != {:?}", rhs_hash, lhs_hash).to_string(),
        ));
    }

    let GetIASSocketResult { fd } = get_ias_socket().map_err(Error::host_api)?;

    let (attn_report, signature, signing_cert) =
        get_report_from_intel(fd, quote, api_hex_str_bytes);

    Ok(EndorsedAttestationVerificationReport {
        avr: attn_report,
        signature,
        signing_cert,
    })
}

pub fn get_sigrl_from_intel(fd: c_int, gid: u32, ias_key: &[u8]) -> Vec<u8> {
    trace!("get_sigrl_from_intel fd = {:?}", fd);
    let config = make_ias_client_config();
    let ias_key = String::from_utf8_lossy(ias_key).trim_end().to_owned();

    let req = format!("GET {}{:08x} HTTP/1.1\r\nHOST: {}\r\nOcp-Apim-Subscription-Key: {}\r\nConnection: Close\r\n\r\n",
                      SIGRL_SUFFIX,
                      gid,
                      DEV_HOSTNAME,
                      ias_key);

    trace!("get_sigrl_from_intel: {}", req);

    let dns_name = webpki::DNSNameRef::try_from_ascii_str(DEV_HOSTNAME).unwrap();
    let mut sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
    let mut sock = TcpStream::new(fd).unwrap();
    let mut tls = rustls::Stream::new(&mut sess, &mut sock);

    let _result = tls.write(req.as_bytes());
    let mut plaintext = Vec::new();

    info!("write complete");

    match tls.read_to_end(&mut plaintext) {
        Ok(_) => (),
        Err(e) => {
            warn!("get_sigrl_from_intel tls.read_to_end: {:?}", e);
            panic!("Communication error with IAS");
        }
    }
    info!("read_to_end complete");
    let resp_string = String::from_utf8(plaintext.clone()).unwrap();

    trace!("{}", resp_string);

    // resp_string

    parse_response_sigrl(&plaintext)
}

// TODO: support pse
pub fn get_report_from_intel(
    fd: c_int,
    quote: Vec<u8>,
    ias_key: &[u8],
) -> (String, Vec<u8>, Vec<u8>) {
    trace!("get_report_from_intel fd = {:?}", fd);
    let config = make_ias_client_config();
    let encoded_quote = base64::encode(&quote[..]);
    let encoded_json = format!("{{\"isvEnclaveQuote\":\"{}\"}}\r\n", encoded_quote);
    let ias_key = String::from_utf8_lossy(ias_key).trim_end().to_owned();

    let req = format!("POST {} HTTP/1.1\r\nHOST: {}\r\nOcp-Apim-Subscription-Key:{}\r\nContent-Length:{}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{}",
                      REPORT_SUFFIX,
                      DEV_HOSTNAME,
                      ias_key,
                      encoded_json.len(),
                      encoded_json);

    trace!("{}", req);
    let dns_name = webpki::DNSNameRef::try_from_ascii_str(DEV_HOSTNAME).unwrap();
    let mut sess = rustls::ClientSession::new(&Arc::new(config), dns_name);
    let mut sock = TcpStream::new(fd).unwrap();
    let mut tls = rustls::Stream::new(&mut sess, &mut sock);

    let _result = tls.write(req.as_bytes());
    let mut plaintext = Vec::new();

    info!("write complete");

    tls.read_to_end(&mut plaintext).unwrap();
    info!("read_to_end complete");
    let resp_string = String::from_utf8(plaintext.clone()).unwrap();

    trace!("resp_string = {}", resp_string);

    let (attn_report, sig, cert) = parse_response_attn_report(&plaintext);

    (attn_report, sig, cert)
}

fn parse_response_attn_report(resp: &[u8]) -> (String, Vec<u8>, Vec<u8>) {
    trace!("parse_response_attn_report");
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut respp = httparse::Response::new(&mut headers);
    let result = respp.parse(resp);
    trace!("parse result {:?}", result);

    let msg: &'static str;

    match respp.code {
        Some(200) => msg = "OK Operation Successful",
        Some(401) => msg = "Unauthorized Failed to authenticate or authorize request.",
        Some(404) => msg = "Not Found GID does not refer to a valid EPID group ID.",
        Some(500) => msg = "Internal error occurred",
        Some(503) => {
            msg = "Service is currently not able to process the request (due to
            a temporary overloading or maintenance). This is a
            temporary state – the same request can be repeated after
            some time. "
        }
        _ => {
            warn!("DBG:{}", respp.code.unwrap());
            msg = "Unknown error occured"
        }
    }

    info!("{}", msg);
    let mut len_num: u32 = 0;

    let mut sig = String::new();
    let mut cert = String::new();
    let mut attn_report = String::new();

    for i in 0..respp.headers.len() {
        let h = respp.headers[i];
        match h.name {
            "Content-Length" => {
                let len_str = String::from_utf8(h.value.to_vec()).unwrap();
                len_num = len_str.parse::<u32>().unwrap();
                trace!("content length = {}", len_num);
            }
            "X-IASReport-Signature" => sig = str::from_utf8(h.value).unwrap().to_string(),
            "X-IASReport-Signing-Certificate" => {
                cert = str::from_utf8(h.value).unwrap().to_string()
            }
            _ => (),
        }
    }

    // Remove %0A from cert, and only obtain the signing cert
    cert = cert.replace("%0A", "");
    cert = percent_decode(cert);

    let v: Vec<&str> = cert.split("-----").collect();
    let sig_cert = v[2].to_string();

    if len_num != 0 {
        let header_len = result.unwrap().unwrap();
        let resp_body = &resp[header_len..];
        attn_report = str::from_utf8(resp_body).unwrap().to_string();
        info!("Attestation report: {}", attn_report);
    }

    let sig_bytes = base64::decode(&sig).unwrap();
    let sig_cert_bytes = base64::decode(&sig_cert).unwrap();
    // len_num == 0
    (attn_report, sig_bytes, sig_cert_bytes)
}

fn parse_response_sigrl(resp: &[u8]) -> Vec<u8> {
    trace!("parse_response_sigrl");
    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut respp = httparse::Response::new(&mut headers);
    let result = respp.parse(resp);
    trace!("parse result {:?}", result);
    trace!("parse response{:?}", respp);

    let msg: &'static str;

    match respp.code {
        Some(200) => msg = "OK Operation Successful",
        Some(401) => msg = "Unauthorized Failed to authenticate or authorize request.",
        Some(404) => msg = "Not Found GID does not refer to a valid EPID group ID.",
        Some(500) => msg = "Internal error occurred",
        Some(503) => {
            msg = "Service is currently not able to process the request (due to
            a temporary overloading or maintenance). This is a
            temporary state – the same request can be repeated after
            some time. "
        }
        _ => msg = "Unknown error occured",
    }

    info!("{}", msg);
    let mut len_num: u32 = 0;

    for i in 0..respp.headers.len() {
        let h = respp.headers[i];
        if h.name == "content-length" {
            let len_str = String::from_utf8(h.value.to_vec()).unwrap();
            len_num = len_str.parse::<u32>().unwrap();
            trace!("content length = {}", len_num);
        }
    }

    if len_num != 0 {
        let header_len = result.unwrap().unwrap();
        let resp_body = &resp[header_len..];
        trace!("Base64-encoded SigRL: {:?}", resp_body);

        return base64::decode(str::from_utf8(resp_body).unwrap()).unwrap();
    }

    // len_num == 0
    Vec::new()
}

pub fn make_ias_client_config() -> rustls::ClientConfig {
    let mut config = rustls::ClientConfig::new();

    config
        .root_store
        .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);

    config
}

fn as_u32_le(array: &[u8; 4]) -> u32 {
    ((array[0] as u32) << 0)
        + ((array[1] as u32) << 8)
        + ((array[2] as u32) << 16)
        + ((array[3] as u32) << 24)
}

fn percent_decode(orig: String) -> String {
    let v: Vec<&str> = orig.split("%").collect();
    let mut ret = String::new();
    ret.push_str(v[0]);
    if v.len() > 1 {
        for s in v[1..].iter() {
            ret.push(u8::from_str_radix(&s[0..2], 16).unwrap() as char);
            ret.push_str(&s[2..]);
        }
    }
    ret
}
