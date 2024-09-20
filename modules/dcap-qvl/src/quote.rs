use alloc::string::String;
use alloc::vec::Vec;

use anyhow::{anyhow, Result};
use scale::{Decode, Input};

use crate::{constants::*, utils, Error};

#[derive(Debug, Clone)]
pub struct Data<T> {
    pub data: Vec<u8>,
    _marker: core::marker::PhantomData<T>,
}

impl<T: Decode + Into<u64>> Decode for Data<T> {
    fn decode<I: Input>(input: &mut I) -> Result<Self, scale::Error> {
        let len = T::decode(input)?;
        let mut data = vec![0u8; len.into() as usize];
        input.read(&mut data)?;
        Ok(Data {
            data,
            _marker: core::marker::PhantomData,
        })
    }
}

#[derive(Decode, Debug)]
pub struct Header {
    pub version: u16,
    pub attestation_key_type: u16,
    pub tee_type: u32,
    pub qe_svn: u16,
    pub pce_svn: u16,
    pub qe_vendor_id: [u8; 16],
    pub user_data: [u8; 20],
}

#[derive(Decode, Debug)]
pub struct Body {
    pub body_type: u16,
    pub size: u32,
}

#[derive(Decode, Debug, Clone)]
pub struct EnclaveReport {
    pub cpu_svn: [u8; 16],
    pub misc_select: u32,
    pub reserved1: [u8; 28],
    pub attributes: [u8; 16],
    pub mr_enclave: [u8; 32],
    pub reserved2: [u8; 32],
    pub mr_signer: [u8; 32],
    pub reserved3: [u8; 96],
    pub isv_prod_id: u16,
    pub isv_svn: u16,
    pub reserved4: [u8; 60],
    pub report_data: [u8; 64],
}

#[derive(Decode, Debug, Clone)]
pub struct TDReport10 {
    pub tee_tcb_svn: [u8; 16],
    pub mr_seam: [u8; 48],
    pub mr_signer_seam: [u8; 48],
    pub seam_attributes: [u8; 8],
    pub td_attributes: [u8; 8],
    pub xfam: [u8; 8],
    pub mr_td: [u8; 48],
    pub mr_config_id: [u8; 48],
    pub mr_owner: [u8; 48],
    pub mr_owner_config: [u8; 48],
    pub rt_mr0: [u8; 48],
    pub rt_mr1: [u8; 48],
    pub rt_mr2: [u8; 48],
    pub rt_mr3: [u8; 48],
    pub report_data: [u8; 64],
}

#[derive(Decode, Debug, Clone)]
pub struct TDReport15 {
    pub base: TDReport10,
    pub tee_tcb_svn2: [u8; 16],
    pub mr_service_td: [u8; 48],
}

#[derive(Decode)]
pub struct CertificationData {
    pub cert_type: u16,
    pub body: Data<u32>,
}

impl core::fmt::Debug for CertificationData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let body_str = String::from_utf8_lossy(&self.body.data);
        f.debug_struct("CertificationData")
            .field("cert_type", &self.cert_type)
            .field("body", &body_str)
            .finish()
    }
}

#[derive(Decode, Debug)]
pub struct QEReportCertificationData {
    pub qe_report: [u8; ENCLAVE_REPORT_BYTE_LEN],
    pub qe_report_signature: [u8; QE_REPORT_SIG_BYTE_LEN],
    pub qe_auth_data: Data<u16>,
    pub certification_data: CertificationData,
}

#[derive(Decode, Debug)]
pub struct AuthDataV3 {
    pub ecdsa_signature: [u8; ECDSA_SIGNATURE_BYTE_LEN],
    pub ecdsa_attestation_key: [u8; ECDSA_PUBKEY_BYTE_LEN],
    pub qe_report: [u8; ENCLAVE_REPORT_BYTE_LEN],
    pub qe_report_signature: [u8; QE_REPORT_SIG_BYTE_LEN],
    pub qe_auth_data: Data<u16>,
    pub certification_data: CertificationData,
}

#[derive(Debug)]
pub struct AuthDataV4 {
    pub ecdsa_signature: [u8; ECDSA_SIGNATURE_BYTE_LEN],
    pub ecdsa_attestation_key: [u8; ECDSA_PUBKEY_BYTE_LEN],
    pub certification_data: CertificationData,
    pub qe_report_data: QEReportCertificationData,
}

impl AuthDataV4 {
    pub fn into_v3(self) -> AuthDataV3 {
        AuthDataV3 {
            ecdsa_signature: self.ecdsa_signature,
            ecdsa_attestation_key: self.ecdsa_attestation_key,
            qe_report: self.qe_report_data.qe_report,
            qe_report_signature: self.qe_report_data.qe_report_signature,
            qe_auth_data: self.qe_report_data.qe_auth_data,
            certification_data: self.qe_report_data.certification_data,
        }
    }
}

impl Decode for AuthDataV4 {
    fn decode<I: Input>(input: &mut I) -> Result<Self, scale::Error> {
        let ecdsa_signature = Decode::decode(input)?;
        let ecdsa_attestation_key = Decode::decode(input)?;
        let certification_data: CertificationData = Decode::decode(input)?;
        let qe_report_data =
            QEReportCertificationData::decode(&mut &certification_data.body.data[..])?;
        Ok(AuthDataV4 {
            ecdsa_signature,
            ecdsa_attestation_key,
            certification_data,
            qe_report_data,
        })
    }
}

#[derive(Debug)]
pub enum AuthData {
    V3(AuthDataV3),
    V4(AuthDataV4),
}

impl AuthData {
    pub fn into_v3(self) -> AuthDataV3 {
        match self {
            AuthData::V3(data) => data,
            AuthData::V4(data) => data.into_v3(),
        }
    }
}

fn decode_auth_data(ver: u16, input: &mut &[u8]) -> Result<AuthData, scale::Error> {
    match ver {
        3 => {
            let auth_data = AuthDataV3::decode(input)?;
            Ok(AuthData::V3(auth_data))
        }
        4 => {
            let auth_data = AuthDataV4::decode(input)?;
            Ok(AuthData::V4(auth_data))
        }
        _ => Err(scale::Error::from("unsupported quote version")),
    }
}

#[derive(Debug, Clone)]
pub enum Report {
    SgxEnclave(EnclaveReport),
    TD10(TDReport10),
    TD15(TDReport15),
}

#[derive(Debug)]
pub struct Quote {
    pub header: Header,
    pub report: Report,
    pub auth_data: AuthData,
}

impl Decode for Quote {
    fn decode<I: Input>(input: &mut I) -> Result<Self, scale::Error> {
        let header = Header::decode(input)?;
        let report;
        match header.version {
            3 => {
                if header.tee_type != TEE_TYPE_SGX {
                    return Err(scale::Error::from("invalid tee type"));
                }
                report = Report::SgxEnclave(EnclaveReport::decode(input)?);
            }
            4 => match header.tee_type {
                TEE_TYPE_SGX => {
                    report = Report::SgxEnclave(EnclaveReport::decode(input)?);
                }
                TEE_TYPE_TDX => {
                    report = Report::TD10(TDReport10::decode(input)?);
                }
                _ => return Err(scale::Error::from("invalid tee type")),
            },
            5 => {
                let body = Body::decode(input)?;
                match body.body_type {
                    BODY_SGX_ENCLAVE_REPORT_TYPE => {
                        report = Report::SgxEnclave(EnclaveReport::decode(input)?);
                    }
                    BODY_TD_REPORT10_TYPE => {
                        report = Report::TD10(TDReport10::decode(input)?);
                    }
                    BODY_TD_REPORT15_TYPE => {
                        report = Report::TD15(TDReport15::decode(input)?);
                    }
                    _ => return Err(scale::Error::from("unsupported body type")),
                }
            }
            _ => return Err(scale::Error::from("unsupported quote version")),
        }
        let data = Data::<u32>::decode(input)?;
        let auth_data = decode_auth_data(header.version, &mut &data.data[..])?;
        Ok(Quote {
            header,
            report,
            auth_data,
        })
    }
}

impl Quote {
    /// Parse a TEE quote from a byte slice.
    pub fn parse(quote: &[u8]) -> Result<Self> {
        let mut input = quote;
        let quote =
            Quote::decode(&mut input).map_err(|e| anyhow!("Failed to parse TEE quote: {:?}", e))?;
        Ok(quote)
    }

    /// Get the raw certificate chain from the quote.
    pub fn raw_cert_chain(&self) -> &[u8] {
        match &self.auth_data {
            AuthData::V3(data) => &data.certification_data.body.data,
            AuthData::V4(data) => &data.qe_report_data.certification_data.body.data,
        }
    }

    /// Get the FMSPC from the quote.
    pub fn fmspc(&self) -> Result<Fmspc, Error> {
        let raw_cert_chain = self.raw_cert_chain();
        let certs = utils::extract_certs(raw_cert_chain)?;
        let extension_section = utils::get_intel_extension(&certs[0])?;
        utils::get_fmspc(&extension_section)
    }

    /// Get the the length of signed data in the quote.
    pub fn signed_length(&self) -> usize {
        let mut len = match self.report {
            Report::SgxEnclave(_) => HEADER_BYTE_LEN + ENCLAVE_REPORT_BYTE_LEN,
            Report::TD10(_) => HEADER_BYTE_LEN + TD_REPORT10_BYTE_LEN,
            Report::TD15(_) => HEADER_BYTE_LEN + TD_REPORT15_BYTE_LEN,
        };
        if self.header.version == 5 {
            len += BODY_BYTE_SIZE;
        }
        len
    }
}
