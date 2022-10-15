use crate::prelude::*;
use flex_error::*;
use light_client::LightClientInstanceError;

define_error! {
    #[derive(Debug, PartialEq, Eq)]
    Error {
        UnexpectedClientType {
            type_url: String
        }
        |e| {
            format_args!("unexpected client_type error: type_url={}", e.type_url)
        },

        Ics02
        [ibc::core::ics02_client::error::Error]
        |_| { "ICS02 client error" },

        Ics03
        [ibc::core::ics03_connection::error::Error]
        |_| { "ICS03 connection error" },

        Ics04
        [ibc::core::ics04_channel::error::Error]
        |_| { "ICS04 channel error" },

        Ics23
        [ibc::core::ics23_commitment::error::Error]
        |_| { "ICS23 commitment error" },

        IbcProof
        [ibc::proofs::ProofError]
        |_| { "IBC Proof error" },

        Commitment
        [commitments::Error]
        |_| { "Commitment error" }
    }
}

impl LightClientInstanceError for Error {}
