use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        TypeUrlNotFound
        {
            type_url: String
        }
        |e| {
            format_args!("type_url not found: type_url={}", e.type_url)
        },

        TypeUrlAlreadyExists
        {
            type_url: String
        }
        |e| {
            format_args!("type_url already exists: type_url={}", e.type_url)
        },

        AlreadySealed
        |_| { "registry is already sealed" },
    }
}
