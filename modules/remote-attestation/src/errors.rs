use attestation_report::QEType;
use dcap_quote_verifier::verifier::Status;
use flex_error::*;
use lcp_types::Time;
use sgx_types::{sgx_quote3_error_t, sgx_status_t};

define_error! {
    #[derive(Debug)]
    Error {
        InvalidSpid {
            descr: String
        }
        |e| {
            format_args!("Invalid SPID: {}", e.descr)
        },

        InvalidUtf8Bytes {
            bytes: Vec<u8>,
            error: std::str::Utf8Error,
            descr: String,
        }
        |e| {
            format_args!("Invalid utf8: bytes={:?} descr={}", e.bytes, e.descr)
        },

        InvalidU32String {
            string: String,
            error: std::num::ParseIntError,
            descr: String,
        }
        |e| {
            format_args!("Invalid u32 string: string={} descr={}", e.string, e.descr)
        },

        Base64Decode {
            error: base64::DecodeError,
            descr: String,
        }
        |e| {
            format_args!("Base64Decode: descr={}", e.descr)
        },

        InvalidPercentDecode {
            value: String,
        }
        |e| {
            format_args!("Invalid percent decode: value={}", e.value)
        },

        IoError {
            error: std::io::Error,
            descr: String,
        }
        |e| {
            format_args!("IOError: error={:?} descr={}", e.error, e.descr)
        },

        Rustls
        [TraceError<rustls::Error>]
        |_| { "Rustls error" },

        InvalidIasServerName
        |_| {
            format_args!("Invalid IAS server name")
        },

        HttpParseError
        [TraceError<httparse::Error>]
        |_| { "HttpParseError" },

        Reqwest
        [TraceError<reqwest::Error>]
        |_| { "Reqwest error" },

        ReqwestGet
        [TraceError<reqwest::Error>]
        |_| { "Reqwest get error" },

        InvalidHttpStatus {
            url: String,
            status: reqwest::StatusCode,
        }
        |e| {
            format_args!("Invalid HTTP status: url={} status={}", e.url, e.status)
        },

        Pem
        [TraceError<pem::PemError>]
        |_| { "Pem error" },

        HttpParsePartialStatus
        |_| { "HttpParsePartialStatus" },

        CannotLookupAddress {
            host: String,
            port: u16,
        }
        |e| {
            format_args!("Cannot lookup address: host={} port={}", e.host, e.port)
        },

        TooOldReportTimestamp {
            now: Time,
            timestamp: Time
        }
        |e| {
            format_args!("Too old report timestamp: the timestamp of the report is too old: now={:?} attestation_time={:?}", e.now, e.timestamp)
        },

        AttestationReport
        [attestation_report::Error]
        |_| { "AttestationReport error" },

        KeyManager
        {
            descr: String
        }
        [keymanager::Error]
        |e| {
            format_args!("KeyManager error: descr={}", e.descr)
        },

        UnexpectedIasReportResponse {
            descr: String
        }
        |e| {
            format_args!("Unexpected IAS Report response error: {}", e.descr)
        },

        UnexpectedSigrlResponse {
            descr: String
        }
        |e| {
            format_args!("Unexpected sigrl response error: {}", e.descr)
        },

        UnexpectedIasReportCertificateResponse {
            descr: String
        }
        |e| {
            format_args!("Unexpected IAS certificate response error: {}", e.descr)
        },

        UnexpectedReport {
            descr: String
        }
        |e| {
            format_args!("Unexpected report error: {}", e.descr)
        },

        UnexpectedQuote {
            descr: String
        }
        |e| {
            format_args!("Unexpected quote error: {}", e.descr)
        },

        UnexpectedQeType {
            expected: QEType,
            actual: QEType,
        }
        |e| {
            format_args!("Unexpected QE type: expected={:?} actual={:?}", e.expected, e.actual)
        },

        TcbStatusNotAllowed {
            tcb_status: Status,
        }
        |e| {
            format_args!("TCB status Not allowed: tcb_status={:?}", e.tcb_status)
        },

        AdvisoryIdsNotAllowed {
            advisory_ids: Vec<String>,
        }
        |e| {
            format_args!("Advisory Ids Not allowed: advisory_ids={:?}", e.advisory_ids)
        },

        SgxError {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("SGXError: status={:?} descr={}", e.status, e.descr)
        },

        SgxQe3Error {
            status: sgx_quote3_error_t,
            descr: String
        }
        |e| {
            format_args!("SGXQE3Error: status={:?} descr={}", e.status, e.descr)
        },

        Time
        [lcp_types::TimeError]
        |_| { "Time error" },

        Zkvm
        [zkvm::Error]
        |_| { "Zkvm error" },

        WebPki
        {
            descr: String
        }
        |e| {
            format_args!("WebPKI error: descr={}", e.descr)
        },

        Collateral
        {
            descr: String
        }
        |e| {
            format_args!("Collateral: descr={}", e.descr)
        },

        X509CertFromPem
        {
            pem: String,
            descr: String
        }
        |e| {
            format_args!("X509CertFromPem: pem={} descr={}", e.pem, e.descr)
        },

        EcPrivateKeyFromPem
        {
            pem: String,
            descr: String
        }
        |e| {
            format_args!("EcPrivateKeyFromPem: pem={} descr={}", e.pem, e.descr)
        },

        DcapQuoteVerifier
        [TraceError<dcap_quote_verifier::Error>]
        |_| { "DCAP quote verifier error" },

        Anyhow
        [TraceError<anyhow::Error>]
        |_| { "Anyhow error" },
    }
}

impl From<attestation_report::Error> for Error {
    fn from(e: attestation_report::Error) -> Self {
        Error::attestation_report(e)
    }
}

impl From<lcp_types::TimeError> for Error {
    fn from(e: lcp_types::TimeError) -> Self {
        Error::time(e)
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::reqwest(e)
    }
}

impl From<zkvm::Error> for Error {
    fn from(e: zkvm::Error) -> Self {
        Error::zkvm(e)
    }
}
