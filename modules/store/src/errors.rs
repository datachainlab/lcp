use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, Clone, PartialEq, Eq)]
    Error {
        Commit {
            descr: String
        }
        |e| {
            format_args!("Commit error: {}", e.descr)
        },
    }
}
