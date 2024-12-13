use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        MissingBonsaiApiKey
        | _ | { "Missing Bonsai API key" },

        LocalProvingError
        {
            descr: String
        }
        |e| {
            format_args!("Local proving error: {}", e.descr)
        },

        BonsaiProvingError
        {
            descr: String
        }
        |e| {
            format_args!("Bonsai proving error: {}", e.descr)
        },

        UnsupportedReceiptType
        {
            descr: String
        }
        |e| {
            format_args!("Unsupported receipt type: {}", e.descr)
        },

        Groth16VerificationError
        {
            descr: String
        }
        |e| {
            format_args!("Groth16 verification error: {}", e.descr)
        },

        UnexpectedSelector
        {
            expected: Vec<u8>,
            actual: Vec<u8>
        }
        |e| {
            format_args!("Expected selector {:?} but got {:?}", e.expected, e.actual)
        }
    }
}
