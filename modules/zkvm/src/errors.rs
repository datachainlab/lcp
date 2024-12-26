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
    }
}
