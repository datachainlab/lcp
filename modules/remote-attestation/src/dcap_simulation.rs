use crate::dcap_utils::DCAPRemoteAttestationResult;
use crate::errors::Error;
use attestation_report::QEType;
use crypto::Address;
use dcap_collaterals::certs::{gen_crl, gen_pck_certchain, PckCa, RootCa};
use dcap_collaterals::enclave_identity::{EnclaveIdentityId, EnclaveIdentityV2Builder};
use dcap_collaterals::enclave_report::{
    build_qe_auth_data, build_qe_report_data, EnclaveReportBuilder,
};
use dcap_collaterals::openssl::pkey::{PKey, Private};
use dcap_collaterals::openssl::x509::X509;
use dcap_collaterals::quote::{
    build_qe_cert_data, gen_quote_v3, sign_qe_report, QuoteHeaderBuilder,
};
use dcap_collaterals::tcbinfo::{
    TcbInfoV3Builder, TcbInfoV3TcbLevelBuilder, TcbInfoV3TcbLevelItemBuilder,
};
use dcap_collaterals::utils::{gen_key, p256_prvkey_to_pubkey_bytes};
use dcap_collaterals::{certs::gen_tcb_certchain, sgx_extensions::SgxExtensionsBuilder};
use dcap_quote_verifier::quotes::version_3::verify_quote_dcapv3;
use dcap_quote_verifier::types::quotes::body::EnclaveReport;
use dcap_quote_verifier::types::quotes::version_3::QuoteV3;
use dcap_quote_verifier::{collaterals::IntelCollateral, types::cert::SgxExtensionTcbLevel};
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use log::*;
use serde_json::json;
use sgx_types::{sgx_report_body_t, sgx_report_t};

#[derive(Debug, Clone)]
pub struct DCAPRASimulationOpts {
    root_cert: X509,
    root_key: PKey<Private>,
}

impl DCAPRASimulationOpts {
    pub fn build_from_pems(root_cert_pem: &[u8], root_key_pem: &[u8]) -> Result<Self, Error> {
        let root_cert = X509::from_pem(root_cert_pem).map_err(|e| {
            Error::x509_cert_from_pem(
                String::from_utf8_lossy(root_cert_pem).to_string(),
                e.to_string(),
            )
        })?;
        let root_key = PKey::<Private>::private_key_from_pem(root_key_pem).map_err(|e| {
            Error::ec_private_key_from_pem(
                String::from_utf8_lossy(root_key_pem).to_string(),
                e.to_string(),
            )
        })?;
        Ok(Self {
            root_cert,
            root_key,
        })
    }

    pub fn root_cert(&self) -> &X509 {
        &self.root_cert
    }

    pub fn root_key(&self) -> &PKey<Private> {
        &self.root_key
    }
}

pub fn run_dcap_ra_simulation(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    opts: DCAPRASimulationOpts,
) -> Result<(), Error> {
    let current_time = Time::now();
    let res = dcap_ra_simulation(key_manager, target_enclave_key, current_time, opts)?;

    key_manager
        .save_ra_quote(target_enclave_key, res.get_ra_quote(current_time).into())
        .map_err(|e| {
            Error::key_manager(
                format!("cannot save DCAP Simulation Quote: {}", target_enclave_key),
                e,
            )
        })?;
    Ok(())
}

pub(crate) fn dcap_ra_simulation(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    current_time: Time,
    opts: DCAPRASimulationOpts,
) -> Result<DCAPRemoteAttestationResult, Error> {
    let ek_info = key_manager.load(target_enclave_key).map_err(|e| {
        Error::key_manager(
            format!("cannot load enclave key: {}", target_enclave_key),
            e,
        )
    })?;
    if ek_info.qe_type != QEType::QE3SIM {
        return Err(Error::unexpected_qe_type(QEType::QE3SIM, ek_info.qe_type));
    }

    let (quote, collateral) = simulate_gen_quote_and_collaterals(&ek_info.report, opts)?;

    debug!(
        "DCAP RA simulation done: quote={:?} collateral={:?}",
        quote, collateral
    );

    let output = verify_quote_dcapv3(&quote, &collateral, current_time.as_unix_timestamp_secs())
        .map_err(Error::dcap_quote_verifier)?;

    Ok(DCAPRemoteAttestationResult {
        raw_quote: quote.to_bytes(),
        output,
        collateral,
    })
}

pub(crate) fn simulate_gen_quote_and_collaterals(
    isv_enclave_report: &sgx_report_t,
    opts: DCAPRASimulationOpts,
) -> Result<(QuoteV3, IntelCollateral), Error> {
    let root_cert = opts.root_cert().clone();
    let root_key = opts.root_key().clone();
    let root_ca = RootCa {
        crl: gen_crl(&root_cert, &root_key, &[], None).unwrap(),
        cert: root_cert,
        key: root_key,
    };

    let tcb_certchain = gen_tcb_certchain(&root_ca, None).unwrap();
    let sgx_extensions = SgxExtensionsBuilder::new()
        .fmspc([0, 96, 106, 0, 0, 0])
        .tcb(SgxExtensionTcbLevel::new(
            &[12, 12, 3, 3, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            13,
            Default::default(),
        ))
        .build();
    let pck_certchain = gen_pck_certchain(
        &root_ca,
        PckCa::Processor,
        &sgx_extensions,
        None,
        None,
        None,
    )
    .unwrap();
    let pck_ca_crl = pck_certchain.pck_cert_crl.to_der().unwrap();

    let quote_header = QuoteHeaderBuilder::new_v3().sgx_tee_type().build();

    let attestation_key = gen_key();

    let qe_cert_data = build_qe_cert_data(
        &pck_certchain.pck_cert,
        &pck_certchain.pck_cert_ca,
        &root_ca.cert,
    );

    let qe_report = EnclaveReportBuilder::new()
        .isv_svn(8)
        .report_data(build_qe_report_data(
            &p256_prvkey_to_pubkey_bytes(&attestation_key).unwrap(),
            build_qe_auth_data(0),
        ))
        .build();

    let qe_report_signature = sign_qe_report(&pck_certchain.pck_cert_key, &qe_report);

    let quote = gen_quote_v3(
        &attestation_key,
        &quote_header,
        to_sgx_enclave_report(isv_enclave_report.body),
        qe_cert_data,
        qe_report,
        qe_report_signature,
    )
    .unwrap();

    let target_tcb_levels = vec![TcbInfoV3TcbLevelItemBuilder::new(
        TcbInfoV3TcbLevelBuilder::new()
            .pcesvn(sgx_extensions.tcb.pcesvn)
            .sgxtcbcomponents(&sgx_extensions.tcb.sgxtcbcompsvns())
            .build(),
    )
    .tcb_status("SWHardeningNeeded")
    .tcb_date_str("2024-03-13T00:00:00Z")
    .advisory_ids(&["INTEL-SA-00334", "INTEL-SA-00615"])
    .build()];

    // fmspc and tcb_levels must be consistent with the sgx extensions in the pck cert
    let tcb_info = TcbInfoV3Builder::new(true)
        .fmspc([0, 96, 106, 0, 0, 0])
        .tcb_levels(target_tcb_levels)
        .build_and_sign(&tcb_certchain.key)
        .unwrap();

    let qe_identity = EnclaveIdentityV2Builder::new(EnclaveIdentityId::QE)
        .tcb_levels_json(json!([
        {
          "tcb": {
            "isvsvn": qe_report.isv_svn
          },
          "tcbDate": "2023-08-09T00:00:00Z",
          "tcbStatus": "UpToDate"
        }
        ]))
        .build_and_sign(&tcb_certchain.key)
        .unwrap();

    let collateral = IntelCollateral {
        tcbinfo_bytes: serde_json::to_vec(&tcb_info).unwrap(),
        qeidentity_bytes: serde_json::to_vec(&qe_identity).unwrap(),
        sgx_intel_root_ca_der: root_ca.cert.to_der().unwrap(),
        sgx_tcb_signing_der: tcb_certchain.cert.to_der().unwrap(),
        sgx_intel_root_ca_crl_der: root_ca.crl.to_der().unwrap(),
        sgx_pck_crl_der: pck_ca_crl,
    };

    Ok((quote, collateral))
}

fn to_sgx_enclave_report(report_body: sgx_report_body_t) -> EnclaveReport {
    let mut attributes = [0u8; 16];
    attributes[..8].copy_from_slice(&report_body.attributes.flags.to_le_bytes());
    attributes[8..].copy_from_slice(&report_body.attributes.xfrm.to_le_bytes());

    EnclaveReportBuilder::new()
        .cpu_svn(report_body.cpu_svn.svn)
        .misc_select(report_body.misc_select.to_le_bytes())
        .attributes(attributes)
        .mrenclave(report_body.mr_enclave.m)
        .mrsigner(report_body.mr_signer.m)
        .isv_prod_id(report_body.isv_prod_id)
        .isv_svn(report_body.isv_svn)
        .report_data(report_body.report_data.d)
        .build()
}
