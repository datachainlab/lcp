use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        HomeDirNotFound
        |_| { "Home directory not found" }
    }
}
