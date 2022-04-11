use derive_more::Display;
use std::string::String;

#[derive(Debug, Display, thiserror::Error)]
pub enum CryptoError {
    /// A key wasn't valid.
    /// e.g. PrivateKey, PublicKey, SharedSecret.
    KeyError,
    /// The random function had failed generating randomness
    RandomError,

    // Errors in contract ABI:
    /// Failed to seal data
    #[display(fmt = "failed to seal data")]
    FailedSeal,
    #[display(fmt = "failed to unseal data")]
    FailedUnseal,

    // An error derived from secp256k1 error
    Secp256k1Error(secp256k1::Error),

    /// An error related to signature verification
    VerificationError(String),
}
