use crypto::Address;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        HomeDirNotFound
        |_| { "Home directory not found" },

        UnattestedEnclaveKey
        {
            address: Address
        }
        |e| {
            format_args!("Unattested enclave key: address={}", e.address)
        },

        UnattestedEnclaveKeyNotFound
        {
            address: Address
        }
        |e| {
            format_args!("Unattested enclave key not found: address={}", e.address)
        },

        Crypto
        [crypto::Error]
        |_| { "Crypto error" },

        AttestationReport
        [attestation_report::Error]
        |_| { "Attestation Report error" },

        Time
        [lcp_types::TimeError]
        |_| { "Time error" },

        Io
        [TraceError<std::io::Error>]
        |_| { "IO error" },

        SerdeJson
        [TraceError<serde_json::Error>]
        |_| { "serde_json error" },

        Rusqlite
        [TraceError<rusqlite::Error>]
        |_| { "rusqlite error" },

        MutexLock
        {
            descr: String
        }
        |e| {
            format_args!("mutex lock error: descr={}", e.descr)
        }
    }
}

impl From<crypto::Error> for Error {
    fn from(value: crypto::Error) -> Self {
        Self::crypto(value)
    }
}

impl From<attestation_report::Error> for Error {
    fn from(value: attestation_report::Error) -> Self {
        Self::attestation_report(value)
    }
}

impl From<lcp_types::TimeError> for Error {
    fn from(value: lcp_types::TimeError) -> Self {
        Self::time(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::serde_json(value)
    }
}

impl From<rusqlite::Error> for Error {
    fn from(value: rusqlite::Error) -> Self {
        Self::rusqlite(value)
    }
}
