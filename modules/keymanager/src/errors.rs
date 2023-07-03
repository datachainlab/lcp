use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        HomeDirNotFound
        |_| { "Home directory not found" },

        Crypto
        {}
        [crypto::Error]
        |_| { "Crypto error" },

        Io
        [TraceError<std::io::Error>]
        |_| { "IO error" },

        TempFilePersist
        [TraceError<tempfile::PersistError>]
        |_| { "failed to persist temp file" },

        SerdeJson
        [TraceError<serde_json::Error>]
        |_| { "serde_json error" },
    }
}

impl From<crypto::Error> for Error {
    fn from(value: crypto::Error) -> Self {
        Self::crypto(value)
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::io(value)
    }
}

impl From<tempfile::PersistError> for Error {
    fn from(value: tempfile::PersistError) -> Self {
        Self::temp_file_persist(value)
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self::serde_json(value)
    }
}
