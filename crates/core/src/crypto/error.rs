use core::fmt::{Display, Formatter, Result};
use std::error::Error;

#[derive(Debug)]
pub enum InvalidPublicKeyError {
    /// The public key is zero.
    PublicKeyIsZero,
    /// The public key modulus the [large safe prime](crate::LARGE_SAFE_PRIME_LITTLE_ENDIAN) is zero.
    PublicKeyModLargeSafePrimeIsZero,
}

impl Error for InvalidPublicKeyError {}
impl Display for InvalidPublicKeyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            InvalidPublicKeyError::PublicKeyIsZero => {
                write!(f, "Public key is zero.")
            }
            InvalidPublicKeyError::PublicKeyModLargeSafePrimeIsZero => {
                write!(f, "Public key modulus the large safe prime is zero.")
            }
        }
    }
}
