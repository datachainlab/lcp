use crate::prelude::*;
use flex_error::*;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        InvalidArgument {
            descr: String
        }
        |e| {
            format_args!("invalid argument: descr={}", e.descr)
        },
        Ics03
        [TraceError<ibc::core::ics03_connection::error::ConnectionError>]
        |_| { "ICS03 connection error" },

        Ics04
        [TraceError<ibc::core::ics04_channel::error::ChannelError>]
        |_| { "ICS04 channel error" },

        Ics24
        [TraceError<ibc::core::ics24_host::error::ValidationError>]
        |_| { "ICS24 host error" }
    }
}
