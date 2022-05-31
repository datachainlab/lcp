use commitments::CommitmentError;
use crypto::CryptoError;
use derive_more::Display;
use light_client::LightClientError;
use ibc::core::ics02_client::error::Error as ICS02Error;

#[derive(thiserror::Error, Debug, Display)]
pub enum LightClientHandlerError {
    ICS02Error(ICS02Error),
    CryptoError(CryptoError),
    LightClientError(LightClientError),
    CommitmentError(CommitmentError),

    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}
