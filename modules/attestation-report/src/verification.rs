use crate::prelude::*;
use crate::{errors::Error, EndorsedAttestationVerificationReport};
use lcp_types::Time;
#[cfg(feature = "sgx")]
use rustls_sgx as rustls;
use tendermint::Time as TmTime;
#[cfg(feature = "sgx")]
use webpki_sgx as webpki;

pub const IAS_REPORT_CA: &[u8] =
    include_bytes!("../../../enclave/Intel_SGX_Attestation_RootCA.pem");

type SignatureAlgorithms = &'static [&'static webpki::SignatureAlgorithm];
static SUPPORTED_SIG_ALGS: SignatureAlgorithms = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

pub fn verify_report(
    report: &EndorsedAttestationVerificationReport,
    current_time: Time,
) -> Result<(), Error> {
    let current_unix_timestamp = current_time.duration_since(TmTime::unix_epoch()).unwrap();
    // NOTE: Currently, webpki::Time's constructor only accepts seconds as unix timestamp.
    // Therefore, the current time are rounded up conservatively.
    let secs = if current_unix_timestamp.subsec_nanos() > 0 {
        current_unix_timestamp.as_secs()
    } else {
        current_unix_timestamp.as_secs() + 1
    };
    let now = webpki::Time::from_seconds_since_unix_epoch(secs);
    let root_ca_pem = pem::parse(IAS_REPORT_CA).expect("failed to parse pem bytes");
    let root_ca = root_ca_pem.contents();

    let mut root_store = rustls::RootCertStore::empty();
    root_store
        .add(&rustls::Certificate(root_ca.to_vec()))
        .map_err(|e| Error::web_pki(e.to_string()))?;

    let trust_anchors: Vec<webpki::TrustAnchor> = root_store
        .roots
        .iter()
        .map(|cert| cert.to_trust_anchor())
        .collect();

    let chain = vec![root_ca];

    let report_cert = webpki::EndEntityCert::from(&report.signing_cert)
        .map_err(|e| Error::web_pki(e.to_string()))?;

    report_cert
        .verify_is_valid_tls_server_cert(
            SUPPORTED_SIG_ALGS,
            &webpki::TLSServerTrustAnchors(&trust_anchors),
            &chain,
            now,
        )
        .map_err(|e| Error::web_pki(e.to_string()))?;

    report_cert
        .verify_signature(
            &webpki::RSA_PKCS1_2048_8192_SHA256,
            report.avr.as_bytes(),
            &report.signature,
        )
        .map_err(|e| Error::web_pki(e.to_string()))?;

    Ok(())
}
