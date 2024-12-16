use crate::errors::Error;
use attestation_report::DCAPQuote;
use crypto::Address;
use dcap_rs::types::collaterals::IntelCollateral;
use dcap_rs::types::quotes::version_3::QuoteV3;
use dcap_rs::utils::cert::{extract_sgx_extension, parse_certchain, parse_pem};
use keymanager::EnclaveKeyManager;
use lcp_types::Time;
use log::*;
use sgx_types::{sgx_qe_get_quote, sgx_qe_get_quote_size, sgx_quote3_error_t, sgx_report_t};

const INTEL_ROOT_CA: &'static [u8] =
    include_bytes!("../assets/Intel_SGX_Provisioning_Certification_RootCA.der");

pub fn run_dcap_ra(
    key_manager: &EnclaveKeyManager,
    target_enclave_key: Address,
) -> Result<(), Error> {
    let ek_info = key_manager.load(target_enclave_key).map_err(|e| {
        Error::key_manager(
            format!("cannot load enclave key: {}", target_enclave_key),
            e,
        )
    })?;
    let raw_quote = rsgx_qe_get_quote(&ek_info.report).unwrap();
    let quote = QuoteV3::from_bytes(&raw_quote);
    println!("Successfully get the quote: {:?}", quote);

    let current_time = Time::now();
    key_manager
        .save_verifiable_quote(
            target_enclave_key,
            DCAPQuote::new(raw_quote, current_time).into(),
        )
        .map_err(|e| {
            Error::key_manager(format!("cannot save DCAP AVR: {}", target_enclave_key), e)
        })?;
    Ok(())
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

async fn get_collateral(pccs_url: &str, quote: &QuoteV3) -> IntelCollateral {
    let base_url = format!("{}/sgx/certification/v4", pccs_url.trim_end_matches('/'));
    info!("base_url: {}", base_url);
    assert_eq!(
        quote.signature.qe_cert_data.cert_data_type, 5,
        "QE Cert Type must be 5"
    );
    let certchain_pems = parse_pem(&quote.signature.qe_cert_data.cert_data).unwrap();
    let certchain = parse_certchain(&certchain_pems);

    // get the pck certificate, and check whether issuer common name is valid
    let pck_cert = &certchain[0];

    // get the SGX extension
    let sgx_extensions = extract_sgx_extension(&pck_cert);
    let fmspc = hex::encode_upper(sgx_extensions.fmspc);

    let client = reqwest::Client::new();
    let mut collateral = IntelCollateral::new();
    {
        let res = client
            .get(format!("{base_url}/tcb?fmspc={fmspc}"))
            .send()
            .await
            .unwrap();
        let issuer_chain = extract_raw_certs(
            get_header(&res, "TCB-Info-Issuer-Chain")
                .unwrap()
                .as_bytes(),
        )
        .unwrap();
        collateral.set_sgx_tcb_signing_der(&issuer_chain[0]);
        collateral.set_tcbinfo_bytes(res.bytes().await.unwrap().as_ref());
    }

    {
        let res = client
            .get(format!("{base_url}/qe/identity"))
            .send()
            .await
            .unwrap();
        collateral.set_qeidentity_bytes(res.bytes().await.unwrap().as_ref());
    }
    collateral.set_intel_root_ca_der(INTEL_ROOT_CA);

    {
        let res = client
            .get("https://certificates.trustedservices.intel.com/IntelSGXRootCA.der")
            .send()
            .await
            .unwrap();
        let crl = res.bytes().await.unwrap();
        collateral.set_sgx_intel_root_ca_crl_der(&crl);
    }

    {
        let res = client
            .get(format!("{base_url}/pckcrl?ca=processor&encoding=der"))
            .send()
            .await
            .unwrap();
        collateral.set_sgx_processor_crl_der(res.bytes().await.unwrap().as_ref());
    }
    {
        let res = client
            .get(format!("{base_url}/pckcrl?ca=platform&encoding=der"))
            .send()
            .await
            .unwrap();
        collateral.set_sgx_platform_crl_der(res.bytes().await.unwrap().as_ref());
    }

    collateral
}

fn get_header(res: &reqwest::Response, name: &str) -> Result<String, String> {
    let value = res
        .headers()
        .get(name)
        .ok_or(format!("Missing {name}"))?
        .to_str()
        .unwrap();
    let value = urlencoding::decode(value).unwrap();
    Ok(value.into_owned())
}

fn extract_raw_certs(cert_chain: &[u8]) -> Result<Vec<Vec<u8>>, Error> {
    Ok(pem::parse_many(cert_chain)
        // .map_err(|_| Error::CodecError)?
        .unwrap()
        .iter()
        .map(|i| i.contents().to_vec())
        .collect())
}

#[cfg(test)]
mod tests {
    use std::time::SystemTime;

    use dcap_rs::utils::quotes::version_3::verify_quote_dcapv3;

    use super::*;

    #[test]
    fn test_quote() {
        QuoteV3::from_bytes(&get_test_quote());
    }

    #[tokio::test]
    async fn test_dcap_collateral() {
        let quote = get_test_quote();
        let quote = QuoteV3::from_bytes(&quote);
        let collateral = get_collateral("https://api.trustedservices.intel.com/", &quote).await;
        let output = verify_quote_dcapv3(
            &quote,
            &collateral,
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        );
        println!("{:?}", output);
    }

    fn get_test_quote() -> Vec<u8> {
        hex::decode("03000200000000000a001000939a7233f79c4ca9940a0db3957f0607b5fe5d7f613d2d40b066b320879bd14d0000000015150b07ff800e000000000000000000000000000000000000000000000000000000000000000000000000000000000005000000000000000700000000000000fe5a6a5bb9128e406517a8bea2d44eb9e238ca581f6a74c11d1437a0e9a6fb12000000000000000000000000000000000000000000000000000000000000000083d719e77deaca1470f6baf62a4d774303c899db69020f9c70ee1dfc08c7ce9e000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000146a5b0a1d467d9dcf154e0b396cf5f44abcfd922000000000000000000000000000000000000000000000000000000000000000000000000000000000000004810000034868d98c393c8e4e470f545383c036336055e5bc04fbff926768b1235c95ab7c369ccf6733fbefd74ee7bd95f60a07bc16e6cad7f35355e3193520ecbf82d934b1526520dd11db5efc9504fa42d048e37ba38c90c8873e7c62f72e86794797bcf8586b9e5c10d0866a95331548da898ae0adf78e428128324151ee558cfc71215150b07ff800e00000000000000000000000000000000000000000000000000000000000000000000000000000000001500000000000000070000000000000096b347a64e5a045e27369c26e6dcda51fd7c850e9b3a3a79e718f43261dee1e400000000000000000000000000000000000000000000000000000000000000008c4f5775d796503e96137f77c68a829a0056ac8ded70140b081b094490c57bff00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001000a0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000017b0dc79c3dc5ff39b3f67346eef41f1ecd63e0a5259a9102eaace1f0aca06ec00000000000000000000000000000000000000000000000000000000000000005ebe66d69491408b1c5948a56b7209b932051148415b68ca371d91ffa4e83e81408e877ac580c5f848a22c849fa4334221695eb4567de369757b949fe086ba7b2000000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f0500e00d00002d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949456a7a4343424453674177494241674956414a34674a3835554b6b7a613873504a4847676e4f4b6d5451426e754d416f4743437147534d343942414d430a4d484578497a416842674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b0a4442464a626e526c6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e560a4241674d416b4e424d517377435159445651514745774a56557a4165467730794e4445794d5445774e44457a4e544e614677307a4d5445794d5445774e44457a0a4e544e614d484178496a416742674e5642414d4d47556c756447567349464e4857434251513073675132567964476c6d61574e6864475578476a415942674e560a42416f4d45556c756447567349454e76636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b470a413155454341774351304578437a414a42674e5642415954416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a304441516344516741450a516a537877644d662b2b3578645553717478343769335952633970504a475434304642774e306e5335557a43314233524b63544875514c3135796b357a4c766c0a5535707a7563552f2b6d674a4e6f55774b6e784942364f434171677767674b6b4d42384741315564497751594d426141464e446f71747031312f6b75535265590a504873555a644456386c6c4e4d477747413155644877526c4d474d77596142666f463247573268306448427a4f693876595842704c6e527964584e305a57527a0a5a584a3261574e6c63793570626e526c6243356a62323076633264344c324e6c636e52705a6d6c6a5958527062323476646a517663474e7259334a7350324e680a5058427962324e6c63334e7663695a6c626d4e765a476c755a7a316b5a584977485159445652304f42425945464f7632356e4f67634c754f693644424b3037470a4d4f5161315a53494d41344741315564447745422f775145417749477744414d42674e5648524d4241663845416a41414d4949423141594a4b6f5a496876684e0a415130424249494278544343416345774867594b4b6f5a496876684e41513042415151514459697469663748386e4277566732482b38504f476a4343415751470a43697147534962345451454e41514977676746554d42414743797147534962345451454e41514942416745564d42414743797147534962345451454e415149430a416745564d42414743797147534962345451454e41514944416745434d42414743797147534962345451454e41514945416745454d42414743797147534962340a5451454e41514946416745424d42454743797147534962345451454e41514947416749416744415142677371686b69472b4530424451454342774942446a41510a42677371686b69472b45304244514543434149424144415142677371686b69472b45304244514543435149424144415142677371686b69472b453042445145430a436749424144415142677371686b69472b45304244514543437749424144415142677371686b69472b45304244514543444149424144415142677371686b69470a2b45304244514543445149424144415142677371686b69472b45304244514543446749424144415142677371686b69472b4530424451454344774942414441510a42677371686b69472b45304244514543454149424144415142677371686b69472b45304244514543455149424454416642677371686b69472b453042445145430a4567515146525543424147414467414141414141414141414144415142676f71686b69472b45304244514544424149414144415542676f71686b69472b4530420a44514545424159416b473756414141774477594b4b6f5a496876684e4151304242516f424144414b42676771686b6a4f5051514441674e4a41444247416945410a326d327a6e44316a3867426453344c74707051445a763246436252686f5a6a46386d474857555637534b34434951447a415847355945585142796d6f4f2b704f0a327a50443978436d2f4f4f794a4f673537574a412f34566f33413d3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436d444343416a36674177494241674956414e446f71747031312f6b7553526559504873555a644456386c6c4e4d416f4743437147534d343942414d430a4d476778476a415942674e5642414d4d45556c756447567349464e48574342536232393049454e424d526f77474159445651514b4442464a626e526c624342440a62334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e424d5173770a435159445651514745774a56557a4165467730784f4441314d6a45784d4455774d5442614677307a4d7a41314d6a45784d4455774d5442614d484578497a41680a42674e5642414d4d476b6c756447567349464e48574342515130736755484a765932567a6332397949454e424d526f77474159445651514b4442464a626e526c0a6243424462334a7762334a6864476c76626a45554d424947413155454277774c553246756447456751327868636d4578437a414a42674e564241674d416b4e420a4d517377435159445651514745774a56557a425a4d424d4742797147534d34394167454743437147534d34394177454841304941424c39712b4e4d7032494f670a74646c31626b2f75575a352b5447516d38614369387a373866732b664b435133642b75447a586e56544154325a68444369667949754a77764e33774e427039690a484253534d4a4d4a72424f6a6762737767626777487759445652306a42426777466f4155496d554d316c71644e496e7a6737535655723951477a6b6e427177770a556759445652306642457377535442486f45576751345a426148523063484d364c79396a5a584a3061575a70593246305a584d7564484a316333526c5a484e6c0a636e5a705932567a4c6d6c75644756734c6d4e766253394a626e526c62464e4857464a76623352445153356b5a584977485159445652304f42425945464e446f0a71747031312f6b7553526559504873555a644456386c6c4e4d41344741315564447745422f77514541774942426a415342674e5648524d4241663845434441470a4151482f416745414d416f4743437147534d343942414d43413067414d4555434951434a6754627456714f795a316d336a716941584d365159613672357357530a34792f4737793875494a4778647749675271507642534b7a7a516167424c517135733541373070646f6961524a387a2f3075447a344e675639316b3d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a2d2d2d2d2d424547494e2043455254494649434154452d2d2d2d2d0a4d4949436a7a4343416a53674177494241674955496d554d316c71644e496e7a6737535655723951477a6b6e42717777436759494b6f5a497a6a3045417749770a614445614d4267474131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e760a636e4276636d4630615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a0a42674e5642415954416c56544d423458445445344d4455794d5445774e4455784d466f58445451354d54497a4d54497a4e546b314f566f77614445614d4267470a4131554541777752535735305a5777675530645949464a766233516751304578476a415942674e5642416f4d45556c756447567349454e76636e4276636d46300a615739754d5251774567594456515148444174545957353059534244624746795954454c4d416b47413155454341774351304578437a414a42674e56424159540a416c56544d466b77457759484b6f5a497a6a3043415159494b6f5a497a6a3044415163445167414543366e45774d4449595a4f6a2f69505773437a61454b69370a314f694f534c52466857476a626e42564a66566e6b59347533496a6b4459594c304d784f346d717379596a6c42616c54565978465032734a424b357a6c4b4f420a757a43427544416642674e5648534d4547444157674251695a517a575770303069664f44744a5653763141624f5363477244425342674e5648523845537a424a0a4d45656752614244686b466f64485277637a6f764c324e6c636e52705a6d6c6a5958526c63793530636e567a6447566b63325679646d6c6a5a584d75615735300a5a577775593239744c306c756447567355306459556d397664454e424c6d526c636a416442674e564851344546675155496d554d316c71644e496e7a673753560a55723951477a6b6e4271777744675944565230504151482f42415144416745474d42494741315564457745422f7751494d4159424166384341514577436759490a4b6f5a497a6a3045417749445351417752674968414f572f35516b522b533943695344634e6f6f774c7550524c735747662f59693747535839344267775477670a41694541344a306c72486f4d732b586f356f2f7358364f39515778485241765a55474f6452513763767152586171493d0a2d2d2d2d2d454e442043455254494649434154452d2d2d2d2d0a00").unwrap()
    }
}