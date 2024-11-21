use crate::prelude::*;
use crate::{errors::Error, SignedAttestationVerificationReport};
use lcp_types::{nanos_to_duration, Time};

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
    current_timestamp: Time,
    report: &SignedAttestationVerificationReport,
) -> Result<(), Error> {
    // NOTE: Currently, webpki::Time's constructor only accepts seconds as unix timestamp.
    // Therefore, the current time are rounded up conservatively.
    let duration = nanos_to_duration(current_timestamp.as_unix_timestamp_nanos())?;
    let secs = if duration.subsec_nanos() > 0 {
        duration.as_secs() + 1
    } else {
        duration.as_secs()
    };
    let now = webpki::Time::from_seconds_since_unix_epoch(secs);
    let root_ca_pem = pem::parse(IAS_REPORT_CA).expect("failed to parse pem bytes");
    let root_ca = root_ca_pem.contents();

    let trust_anchors = vec![webpki::TrustAnchor::try_from_cert_der(root_ca)
        .map_err(|e| Error::web_pki(e.to_string()))?];

    let intermediate_certs = vec![root_ca];
    let report_cert = webpki::EndEntityCert::try_from(report.signing_cert.as_slice())
        .map_err(|e| Error::web_pki(e.to_string()))?;

    report_cert
        .verify_is_valid_tls_server_cert(
            SUPPORTED_SIG_ALGS,
            &webpki::TlsServerTrustAnchors(&trust_anchors),
            &intermediate_certs,
            now,
        )
        .map_err(|e| Error::web_pki(e.to_string()))?;

    report_cert
        .verify_signature(
            &webpki::RSA_PKCS1_2048_8192_SHA256,
            report.avr.as_ref(),
            &report.signature,
        )
        .map_err(|e| Error::web_pki(e.to_string()))?;

    Ok(())
}
