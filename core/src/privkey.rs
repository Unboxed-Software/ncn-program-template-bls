#[cfg(not(target_os = "solana"))]
use rand::RngCore;

use solana_bn254::prelude::alt_bn128_multiplication;

use crate::{error::NCNProgramError, g1_point::G1Point, schemes::HashToCurve};

#[derive(Debug, Clone, Copy)]
pub struct PrivKey(pub [u8; 32]);

impl PrivKey {
    #[cfg(not(target_os = "solana"))]
    pub fn from_random() -> PrivKey {
        use crate::constants::MODULUS;

        loop {
            let mut bytes = [0u8; 32];

            rand::thread_rng().fill_bytes(&mut bytes);

            let num = dashu::integer::UBig::from_be_bytes(&bytes);

            if num < MODULUS {
                return Self(bytes);
            }
        }
    }

    pub fn sign<H: HashToCurve, T: AsRef<[u8]>>(
        &self,
        message: T,
    ) -> Result<G1Point, NCNProgramError> {
        let point = H::try_hash_to_curve::<T>(message)?;

        let input = [&point.0[..], &self.0[..]].concat();

        let mut g1_sol_uncompressed = [0x00u8; 64];
        g1_sol_uncompressed.clone_from_slice(
            &alt_bn128_multiplication(&input).map_err(|_| NCNProgramError::BLSSigningError)?,
        );

        Ok(G1Point(g1_sol_uncompressed))
    }
}

#[cfg(all(test, not(target_os = "solana")))]
mod test {
    use crate::{
        g1_point::{G1CompressedPoint, G1Point},
        g2_point::G2Point,
        schemes::sha256_normalized::Sha256Normalized,
    };

    use super::PrivKey;

    #[test]
    fn sign() {
        let privkey = PrivKey([
            0x21, 0x6f, 0x05, 0xb4, 0x64, 0xd2, 0xca, 0xb2, 0x72, 0x95, 0x4c, 0x66, 0x0d, 0xd4,
            0x5c, 0xf8, 0xab, 0x0b, 0x26, 0x13, 0x65, 0x4d, 0xcc, 0xc7, 0x4c, 0x11, 0x55, 0xfe,
            0xba, 0xaf, 0xb5, 0xc9,
        ]);
        let signature = privkey
            .sign::<Sha256Normalized, &[u8; 6]>(b"sample")
            .expect("Failed to sign");
        assert_eq!(
            [
                0x02, 0x6e, 0x58, 0x71, 0x6e, 0xd0, 0x10, 0x01, 0x81, 0x14, 0x8b, 0x56, 0x47, 0xe8,
                0xf0, 0x79, 0x99, 0xa3, 0x63, 0x99, 0x11, 0x70, 0x95, 0x9e, 0x71, 0x82, 0x80, 0x14,
                0x48, 0x5a, 0xa4, 0x2c, 0x2e, 0xcd, 0x1d, 0xf1, 0x73, 0x22, 0x8c, 0x3f, 0x2a, 0x5c,
                0x9f, 0xd2, 0x0d, 0x44, 0x18, 0xca, 0x6c, 0x10, 0x8b, 0x50, 0xe0, 0x76, 0x63, 0x09,
                0x16, 0xcc, 0x57, 0x0e, 0xc1, 0x5a, 0x77, 0x2a
            ],
            signature.0
        );
    }

    #[test]
    fn sign_random() {
        let message = b"sample";
        let privkey = PrivKey::from_random();
        let signature = privkey
            .sign::<Sha256Normalized, &[u8; 6]>(&message)
            .expect("Failed to sign");
        let pubkey = G2Point::try_from(&privkey).expect("Invalid private key");
        println!("Sig: {:?}\n, Pub: {:?}", &signature.0, &pubkey.0);
        assert!(pubkey
            .verify_signature::<Sha256Normalized, &[u8], G1Point>(signature, message)
            .is_ok());
    }

    #[test]
    fn sign_compressed() {
        let privkey = PrivKey([
            0x21, 0x6f, 0x05, 0xb4, 0x64, 0xd2, 0xca, 0xb2, 0x72, 0x95, 0x4c, 0x66, 0x0d, 0xd4,
            0x5c, 0xf8, 0xab, 0x0b, 0x26, 0x13, 0x65, 0x4d, 0xcc, 0xc7, 0x4c, 0x11, 0x55, 0xfe,
            0xba, 0xaf, 0xb5, 0xc9,
        ]);
        let signature = G1CompressedPoint::try_from(
            privkey
                .sign::<Sha256Normalized, &[u8; 6]>(b"sample")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            [
                0x82, 0x6e, 0x58, 0x71, 0x6e, 0xd0, 0x10, 0x01, 0x81, 0x14, 0x8b, 0x56, 0x47, 0xe8,
                0xf0, 0x79, 0x99, 0xa3, 0x63, 0x99, 0x11, 0x70, 0x95, 0x9e, 0x71, 0x82, 0x80, 0x14,
                0x48, 0x5a, 0xa4, 0x2c
            ],
            signature.0,
        );
    }
}
