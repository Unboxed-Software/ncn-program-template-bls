use crate::{errors::BLSError, g1_point::G1Point};

pub trait HashToCurve {
    /// # Try Hash To Curve
    ///
    /// This trait implements a `try_hash_to_curve` function used to implement signing and verification
    /// of BLS signatures for our custom AltBN128 signature scheme and attempts to return a valid point
    /// in G1 representing our hashed value scalar.
    ///
    /// Consider using this function to implement:
    /// - Hashing algorithm
    /// - Hash scalar normalization
    /// - Domain separation
    fn try_hash_to_curve<T: AsRef<[u8]>>(message: T) -> Result<G1Point, BLSError>;
}

// Trait to represent any type that can be used as a BLS signature
pub trait BLSSignature {
    fn to_bytes(&self) -> Result<[u8; 64], BLSError>;
}
