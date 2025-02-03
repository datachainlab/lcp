use crate::errors::Error;
use attestation_report::DCAPQuote;
use crypto::Address;
use dcap_quote_verifier::types::collaterals::IntelCollateral;
use dcap_quote_verifier::types::quotes::version_3::QuoteV3;
use dcap_quote_verifier::types::VerifiedOutput;
use dcap_quote_verifier::utils::cert::{
    extract_sgx_extensions, get_x509_subject_cn, parse_certchain, parse_pem,
};
use dcap_quote_verifier::utils::quotes::version_3::verify_quote_dcapv3;
use keymanager::EnclaveKeyManager;
use lcp_types::proto::lcp::service::enclave::v1::DcapCollateral;
use lcp_types::Time;
use log::*;
use sgx_types::{sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_quote3_error_t, sgx_report_t};

/// The Intel Root CA certificate in DER format.
/// ref. https://certificates.trustedservices.intel.com/Intel_SGX_Provisioning_Certification_RootCA.cer
pub const INTEL_ROOT_CA: &[u8] =
    include_bytes!("../assets/Intel_SGX_Provisioning_Certification_RootCA.der");

/// The Keccak-256 hash of the Intel Root CA certificate.
/// 0xa1acc73eb45794fa1734f14d882e91925b6006f79d3bb2460df9d01b333d7009
pub const INTEL_ROOT_CA_HASH: [u8; 32] = [
    161, 172, 199, 62, 180, 87, 148, 250, 23, 52, 241, 77, 136, 46, 145, 146, 91, 96, 6, 247, 157,
    59, 178, 70, 13, 249, 208, 27, 51, 61, 112, 9,
];

pub fn run_dcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    pccs_url: &str,
    certs_service_url: &str,
    is_early_update: bool,
) -> Result<(), Error> {
    let current_time = Time::now();
    let result = dcap_ra(
        key_manager,
        target_enclave_key,
        current_time,
        pccs_url,
        certs_service_url,
        is_early_update,
    )?;

    key_manager
        .save_ra_quote(target_enclave_key, result.get_ra_quote(current_time).into())
        .map_err(|e| {
            Error::key_manager(format!("cannot save DCAP quote: {}", target_enclave_key), e)
        })?;
    Ok(())
}

pub(crate) fn dcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
    current_time: Time,
    pccs_url: &str,
    certs_service_url: &str,
    is_early_update: bool,
) -> Result<DCAPRemoteAttestationResult, Error> {
    let ek_info = key_manager.load(target_enclave_key).map_err(|e| {
        Error::key_manager(
            format!("cannot load enclave key: {}", target_enclave_key),
            e,
        )
    })?;
    let raw_quote = rsgx_qe_get_quote(&ek_info.report)
        .map_err(|status| Error::sgx_qe3_error(status, "failed to get quote".into()))?;

    info!("Successfully get the quote: {}", hex::encode(&raw_quote));

    let quote = QuoteV3::from_bytes(&raw_quote).map_err(Error::dcap_quote_verifier)?;

    let collateral = get_collateral(pccs_url, certs_service_url, is_early_update, &quote)?;
    let output = verify_quote_dcapv3(&quote, &collateral, current_time.as_unix_timestamp_secs())
        .map_err(Error::dcap_quote_verifier)?;
    info!(
        "DCAP RA output: {:?} hex={}",
        output,
        hex::encode(output.to_bytes())
    );

    Ok(DCAPRemoteAttestationResult {
        raw_quote,
        output,
        collateral,
    })
}

#[derive(Debug)]
pub struct DCAPRemoteAttestationResult {
    pub raw_quote: Vec<u8>,
    pub output: VerifiedOutput,
    pub collateral: IntelCollateral,
}

impl DCAPRemoteAttestationResult {
    pub fn get_ra_quote(&self, attested_at: Time) -> DCAPQuote {
        DCAPQuote::new(
            self.raw_quote.clone(),
            self.output.fmspc,
            self.output.tcb_status.to_string(),
            self.output.advisory_ids.clone(),
            attested_at,
            DcapCollateral {
                tcbinfo_bytes: self.collateral.tcbinfo_bytes.clone(),
                qeidentity_bytes: self.collateral.qeidentity_bytes.clone(),
                sgx_intel_root_ca_der: self.collateral.sgx_intel_root_ca_der.clone(),
                sgx_tcb_signing_der: self.collateral.sgx_tcb_signing_der.clone(),
                sgx_intel_root_ca_crl_der: self.collateral.sgx_intel_root_ca_crl_der.clone(),
                sgx_pck_crl_der: self.collateral.sgx_pck_crl_der.clone(),
            },
        )
    }
}

fn rsgx_qe_get_quote(app_report: &sgx_report_t) -> Result<Vec<u8>, sgx_quote3_error_t> {
    let mut quote_size = 0;
    unsafe {
        match sgx_qe_get_quote_size(&mut quote_size) {
            sgx_quote3_error_t::SGX_QL_SUCCESS => {
                let mut quote = vec![0u8; quote_size as usize];
                match sgx_qe_get_quote(app_report, quote_size, quote.as_mut_ptr()) {
                    sgx_quote3_error_t::SGX_QL_SUCCESS => Ok(quote),
                    err => Err(err),
                }
            }
            err => Err(err),
        }
    }
}

fn get_collateral(
    pccs_url: &str,
    certs_service_url: &str,
    is_early_update: bool,
    quote: &QuoteV3,
) -> Result<IntelCollateral, Error> {
    let pccs_url = pccs_url.trim_end_matches('/');
    let certs_service_url = certs_service_url.trim_end_matches('/');
    let base_url = format!("{pccs_url}/sgx/certification/v4");
    if quote.signature.qe_cert_data.cert_data_type != 5 {
        return Err(Error::collateral("QE Cert Type must be 5".to_string()));
    }
    let certchain_pems = parse_pem(&quote.signature.qe_cert_data.cert_data)
        .map_err(|e| Error::collateral(format!("cannot parse QE cert chain: {}", e)))?;

    let certchain = parse_certchain(&certchain_pems).map_err(Error::dcap_quote_verifier)?;
    if certchain.len() != 3 {
        return Err(Error::collateral(
            "QE Cert chain must have 3 certs".to_string(),
        ));
    }

    let update_policy = if is_early_update { "early" } else { "standard" };

    // get the pck certificate
    let pck_cert = &certchain[0];
    let pck_cert_issuer = &certchain[1];

    // get the SGX extension
    let sgx_extensions = extract_sgx_extensions(pck_cert);
    let (tcbinfo_bytes, sgx_tcb_signing_der) = {
        let fmspc = hex::encode_upper(sgx_extensions.fmspc);
        let res = http_get(format!(
            "{base_url}/tcb?fmspc={fmspc}&update={update_policy}"
        ))?;
        let issuer_chain =
            extract_raw_certs(get_header(&res, "TCB-Info-Issuer-Chain")?.as_bytes())?;
        (res.bytes()?.to_vec(), issuer_chain[0].clone())
    };

    let qeidentity_bytes = http_get(format!("{base_url}/qe/identity?update={update_policy}"))?
        .bytes()?
        .to_vec();
    let sgx_intel_root_ca_crl_der = http_get(format!("{certs_service_url}/IntelSGXRootCA.der"))?
        .bytes()?
        .to_vec();

    let pck_crl_url = match get_x509_subject_cn(pck_cert_issuer).as_str() {
        "Intel SGX PCK Platform CA" => format!("{base_url}/pckcrl?ca=platform&encoding=der"),
        "Intel SGX PCK Processor CA" => format!("{base_url}/pckcrl?ca=processor&encoding=der"),
        cn => {
            return Err(Error::collateral(format!(
                "Unknown PCK Cert Subject CN: {}",
                cn
            )));
        }
    };
    let sgx_pck_crl_der = http_get(pck_crl_url)?.bytes()?.to_vec();

    Ok(IntelCollateral {
        tcbinfo_bytes,
        qeidentity_bytes,
        sgx_intel_root_ca_der: INTEL_ROOT_CA.to_vec(),
        sgx_tcb_signing_der,
        sgx_intel_root_ca_crl_der,
        sgx_pck_crl_der,
    })
}

fn get_header(res: &reqwest::blocking::Response, name: &str) -> Result<String, Error> {
    let value = res
        .headers()
        .get(name)
        .ok_or_else(|| Error::collateral(format!("missing header {}", name)))?
        .to_str()
        .map_err(|e| Error::collateral(format!("invalid header value: {}", e)))?;
    let value = urlencoding::decode(value)
        .map_err(|e| Error::collateral(format!("invalid header value: {}", e)))?;
    Ok(value.into_owned())
}

fn extract_raw_certs(cert_chain: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    Ok(pem::parse_many(cert_chain)
        .map_err(Error::pem)?
        .iter()
        .map(|i| i.contents().to_vec())
        .collect())
}

fn http_get(url: String) -> Result<reqwest::blocking::Response, Error> {
    info!("get collateral from {}", url);
    let res = reqwest::blocking::get(&url).map_err(Error::reqwest_get)?;
    if !res.status().is_success() {
        return Err(Error::invalid_http_status(url, res.status()));
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dcap_quote_verifier::{
        constants::SGX_TEE_TYPE,
        utils::{hash::keccak256sum, quotes::version_3::verify_quote_dcapv3},
    };

    #[test]
    fn test_quote() {
        assert!(QuoteV3::from_bytes(&get_test_quote()).is_ok());
    }

    #[test]
    fn test_dcap_collateral() {
        let quote = get_test_quote();
        let quote = QuoteV3::from_bytes(&quote).unwrap();
        let collateral = get_collateral(
            "https://api.trustedservices.intel.com/",
            "https://certificates.trustedservices.intel.com/",
            false,
            &quote,
        )
        .unwrap();
        let res = verify_quote_dcapv3(&quote, &collateral, Time::now().as_unix_timestamp_secs());
        assert!(res.is_ok(), "{:?}", res);
        let output = res.unwrap();
        assert_eq!(output.tee_type, SGX_TEE_TYPE);
    }

    fn get_test_quote() -> Vec<u8> {
        hex::decode("03000200000000000a001000939a7233f79c4ca9940a0db3957f0607b5fe5d7f613d2d40b066b320879bd14d0000000015150b07ff800e000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000700000000000000fe5a6a5bb9128e406517a8bea2d44eb9e238ca581f6a74c11d1437a0e9a6fb12000000000000000000000000000000000000000000000000000000000000000083d719e77deaca1470f6baf62a4d774303c899db69020f9c70ee1dfc08c7ce9e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000146a5b0a1d467d9dcf154e0b396cf5f44abcfd922000000000000000000000000000000000000000000000000000000000000000000000000000000000000004810000034868d98c393c8e4e470f545383c036336055e5bc04fbff926768b1235c95ab7c369ccf6733fbefd74ee7bd95f60a07bc16e6cad7f35355e3193520ecbf82d934b1526520dd11db5efc9504fa42d048e37ba38c90c8873e7c62f72e86794797bcf8586b9e5c10d0866a95331548da898ae0adf78e428128324151ee558cfc71215150b07ff800e00000000000000000000000000000000000000000000000000000000000000000000000000000000001500000000000000070000000000000096b347a64e5a045e27369c26e6dcda51fd7c850e9b3a3a79e718f43261dee1e400000000000000000000000000000000000000000000000000000000000000008c4f5775d796503e96137f77c68a829a0056ac8ded70140b081b094490c57bff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000017b0dc79c3dc5ff39b3f67346eef41f1ecd63e0a5259a9102eaace1f0aca06ec00000000000000000000000000000000000000000000000000000000000000005ebe66d69491408b1c5948a56b7209b932051148415b68ca371d91ffa4e83e81408e877ac580c5f848a22c849fa4334221695eb4567de369757b949fe086ba7b2000000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0500e00d00002d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949456a7a4343424453674177494241674956414a34674a3835554b6b7a613873504a4847676e4f4b6d5451426e754d416f4743437147534d343942414d430a4d484578497a416842674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b0a4442464a626e526c6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e560a4241674d416b4e424d517377435159445651514745774a56557a4165467730794e4445794d5445774e44457a4e544e614677307a4d5445794d5445774e44457a0a4e544e614d484178496a416742674e5642414d4d47556c756447567349464e4857434251513073675132567964476c6d61574e6864475578476a415942674e560a42416f4d45556c756447567349454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b470a413155454341774351304578437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741450a516a537877644d662b2b3578645553717478343769335952633970504a475434304642774e306e5335557a43314233524b63544875514c3135796b357a4c766c0a5535707a7563552f2b6d674a4e6f55774b6e784942364f434171677767674b6b4d42384741315564497751594d426141464e446f71747031312f6b75535265590a504873555a644456386c6c4e4d477747413155644877526c4d474d77596142666f463247573268306448427a4f693876595842704c6e527964584e305a57527a0a5a584a3261574e6c63793570626e526c6243356a62323076633264344c324e6c636e52705a6d6c6a5958527062323476646a517663474e7259334a7350324e680a5058427962324e6c63334e7663695a6c626d4e765a476c755a7a316b5a584977485159445652304f42425945464f7632356e4f67634c754f693644424b3037470a4d4f5161315a53494d41344741315564447745422f775145417749477744414d42674e5648524d4241663845416a41414d4949423141594a4b6f5a496876684e0a415130424249494278544343416345774867594b4b6f5a496876684e41513042415151514459697469663748386e4277566732482b38504f476a4343415751470a43697147534962345451454e41514977676746554d42414743797147534962345451454e41514942416745564d42414743797147534962345451454e415149430a416745564d42414743797147534962345451454e41514944416745434d42414743797147534962345451454e41514945416745454d42414743797147534962340a5451454e41514946416745424d42454743797147534962345451454e41514947416749416744415142677371686b69472b4530424451454342774942446a41510a42677371686b69472b45304244514543434149424144415142677371686b69472b45304244514543435149424144415142677371686b69472b453042445145430a436749424144415142677371686b69472b45304244514543437749424144415142677371686b69472b45304244514543444149424144415142677371686b69470a2b45304244514543445149424144415142677371686b69472b45304244514543446749424144415142677371686b69472b4530424451454344774942414441510a42677371686b69472b45304244514543454149424144415142677371686b69472b45304244514543455149424454416642677371686b69472b453042445145430a4567515146525543424147414467414141414141414141414144415142676f71686b69472b45304244514544424149414144415542676f71686b69472b4530420a44514545424159416b473756414141774477594b4b6f5a496876684e4151304242516f424144414b42676771686b6a4f5051514441674e4a41444247416945410a326d327a6e44316a3867426453344c74707051445a763246436252686f5a6a46386d474857555637534b34434951447a415847355945585142796d6f4f2b704f0a327a50443978436d2f4f4f794a4f673537574a412f34566f33413d3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436d444343416a36674177494241674956414e446f71747031312f6b7553526559504873555a644456386c6c4e4d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484578497a41680a42674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b4442464a626e526c0a6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e420a4d517377435159445651514745774a56557a425a4d424d4742797147534d34394167454743437147534d34394177454841304941424c39712b4e4d7032494f670a74646c31626b2f75575a352b5447516d38614369387a373866732b664b435133642b75447a586e56544154325a68444369667949754a77764e33774e427039690a484253534d4a4d4a72424f6a6762737767626777487759445652306a42426777466f4155496d554d316c71644e496e7a6737535655723951477a6b6e427177770a556759445652306642457377535442486f45576751345a426148523063484d364c79396a5a584a3061575a70593246305a584d7564484a316333526c5a484e6c0a636e5a705932567a4c6d6c75644756734c6d4e766253394a626e526c62464e4857464a76623352445153356b5a584977485159445652304f42425945464e446f0a71747031312f6b7553526559504873555a644456386c6c4e4d41344741315564447745422f77514541774942426a415342674e5648524d4241663845434441470a4151482f416745414d416f4743437147534d343942414d43413067414d4555434951434a6754627456714f795a316d336a716941584d365159613672357357530a34792f4737793875494a4778647749675271507642534b7a7a516167424c517135733541373070646f6961524a387a2f3075447a344e675639316b3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00").unwrap()
    }

    #[test]
    fn test_dcap_intel_root_ca_hash() {
        let h = keccak256sum(INTEL_ROOT_CA);
        assert_eq!(h, INTEL_ROOT_CA_HASH);
    }
}
