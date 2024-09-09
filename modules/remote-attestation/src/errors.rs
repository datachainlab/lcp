use flex_error::*;
use lcp_types::Time;
use sgx_types::sgx_status_t;

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
            format_args!("InvalidUtf8: bytes={:?} descr={}", e.bytes, e.descr)
        },

        InvalidU32String {
            string: String,
            error: std::num::ParseIntError,
            descr: String,
        }
        |e| {
            format_args!("InvalidU32String: string={} descr={}", e.string, e.descr)
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
            format_args!("InvalidPercentDecode: value={}", e.value)
        },

        IoError {
            error: std::io::Error,
            descr: String,
        }
        |e| {
            format_args!("IOError: descr={}", e.descr)
        },

        InvalidDnsNameError
        [TraceError<webpki::InvalidDNSNameError>]
        |_| { "InvalidDnsNameError" },

        HttpParseError
        [TraceError<httparse::Error>]
        |_| { "HttpParseError" },

        HttpParsePartialStatus
        |_| { "HttpParsePartialStatus" },

        CannotLookupAddress {
            host: String,
            port: u16,
        }
        |e| {
            format_args!("CannotLookupAddress: host={} port={}", e.host, e.port)
        },

        TooOldReportTimestamp {
            now: Time,
            timestamp: Time
        }
        |e| {
            format_args!("TooOldReportTimestamp: the timestamp of the report is too old: now={:?} attestation_time={:?}", e.now, e.timestamp)
        },

        AttestationReport
        [attestation_report::Error]
        |_| { "AttestationReport error" },

        UnexpectedIasReportResponse {
            descr: String
        }
        |e| {
            format_args!("UnexpectedIASReportResponse error: {}", e.descr)
        },

        UnexpectedSigrlResponse {
            descr: String
        }
        |e| {
            format_args!("UnexpectedSigrlResponse error: {}", e.descr)
        },

        UnexpectedIasReportCertificateResponse {
            descr: String
        }
        |e| {
            format_args!("UnexpectedIASReportCertificateResponse error: {}", e.descr)
        },

        UnexpectedReport {
            descr: String
        }
        |e| {
            format_args!("UnexpectedReport error: {}", e.descr)
        },

        UnexpectedQuote {
            descr: String
        }
        |e| {
            format_args!("UnexpectedQuoteError: {}", e.descr)
        },

        SgxError {
            status: sgx_status_t,
            descr: String
        }
        |e| {
            format_args!("SGXError: status={:?} descr={}", e.status, e.descr)
        },

        Time
        [lcp_types::TimeError]
        |_| { "Time error" },
    }
}
