use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    discriminators::Discriminators,
    error::NCNProgramError,
    g1_point::{G1CompressedPoint, G1Point},
    g2_point::{G2CompressedPoint, G2Point},
    loaders::check_load,
};

/// Individual operator account that stores BLS keys for a specific operator in a specific NCN
#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorEntry {
    /// The NCN this operator entry belongs to
    pub ncn: Pubkey,
    /// The operator pubkey
    pub operator_pubkey: Pubkey,
    /// The G1 pubkey in compressed format (32 bytes)
    pub g1_pubkey: [u8; 32],
    /// The G2 pubkey in compressed format (64 bytes)
    pub g2_pubkey: [u8; 64],
    /// The index of the operator in respect to the NCN account
    pub operator_index: PodU64,
    /// The slot the operator was registered
    pub slot_registered: PodU64,
    /// The bump seed for the PDA
    pub bump: u8,
    /// Reserved for future use
    pub reserved: [u8; 199], // Reserved for future use, must be zeroed
}

impl Discriminator for OperatorEntry {
    const DISCRIMINATOR: u8 = Discriminators::OperatorRegistry as u8;
}

impl OperatorEntry {
    const OPERATOR_ENTRY_SEED: &'static [u8] = b"operator_entry";
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub const EMPTY_OPERATOR_INDEX: u64 = u64::MAX;
    pub const EMPTY_SLOT_REGISTERED: u64 = u64::MAX;

    pub fn new(
        ncn: &Pubkey,
        operator_pubkey: &Pubkey,
        g1_pubkey: &[u8; 32],
        g2_pubkey: &[u8; 64],
        operator_index: u64,
        slot_registered: u64,
        bump: u8,
    ) -> Self {
        Self {
            ncn: *ncn,
            operator_pubkey: *operator_pubkey,
            g1_pubkey: *g1_pubkey,
            g2_pubkey: *g2_pubkey,
            operator_index: PodU64::from(operator_index),
            slot_registered: PodU64::from(slot_registered),
            bump,
            reserved: [0; 199],
        }
    }

    pub fn initialize(
        &mut self,
        ncn: &Pubkey,
        operator_pubkey: &Pubkey,
        g1_pubkey: &[u8; 32],
        g2_pubkey: &[u8; 64],
        operator_index: u64,
        slot_registered: u64,
        bump: u8,
    ) {
        self.ncn = *ncn;
        self.operator_pubkey = *operator_pubkey;
        self.g1_pubkey = *g1_pubkey;
        self.g2_pubkey = *g2_pubkey;
        self.operator_index = PodU64::from(operator_index);
        self.slot_registered = PodU64::from(slot_registered);
        self.bump = bump;
        self.reserved = [0; 199];
    }

    pub fn seeds(ncn: &Pubkey, operator: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            Self::OPERATOR_ENTRY_SEED.to_vec(),
            ncn.to_bytes().to_vec(),
            operator.to_bytes().to_vec(),
        ]
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        operator: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, operator);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        operator: &Pubkey,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn, operator).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }

    pub const fn operator_pubkey(&self) -> &Pubkey {
        &self.operator_pubkey
    }

    pub const fn g1_pubkey(&self) -> &[u8; 32] {
        &self.g1_pubkey
    }

    pub const fn g2_pubkey(&self) -> &[u8; 64] {
        &self.g2_pubkey
    }

    pub fn operator_index(&self) -> u64 {
        self.operator_index.into()
    }

    pub fn slot_registered(&self) -> u64 {
        self.slot_registered.into()
    }

    pub fn is_empty(&self) -> bool {
        self.slot_registered() == Self::EMPTY_SLOT_REGISTERED
    }

    /// Verify that the G1 and G2 keys are related by verifying the pairing
    pub fn verify_keypair(&self) -> Result<(), ProgramError> {
        let g1_compressed = G1CompressedPoint::from(self.g1_pubkey);
        let g2_compressed = G2CompressedPoint::from(self.g2_pubkey);

        // Convert to uncompressed points for verification
        let g1_point = G1Point::try_from(&g1_compressed)
            .map_err(|_| NCNProgramError::G1PointDecompressionError)?;
        let g2_point = G2Point::try_from(g2_compressed)
            .map_err(|_| NCNProgramError::G2PointDecompressionError)?;

        // Verify that g1 and g2 are related (both generated from the same private key)
        g1_point
            .verify_g2(&g2_point)
            .map_err(|_| NCNProgramError::BLSVerificationError)?
            .then_some(())
            .ok_or(ProgramError::from(NCNProgramError::BLSVerificationError))
    }

    /// Update the BLS keys for this operator entry
    pub fn update_keys(
        &mut self,
        new_g1_pubkey: &[u8; 32],
        new_g2_pubkey: &[u8; 64],
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        // Create a temporary entry with new keys to verify them
        let temp_entry = OperatorEntry::new(
            &self.ncn,
            &self.operator_pubkey,
            new_g1_pubkey,
            new_g2_pubkey,
            self.operator_index(),
            current_slot,
            self.bump,
        );

        // Verify the new keypair before updating
        temp_entry.verify_keypair()?;

        // Update the keys
        self.g1_pubkey = *new_g1_pubkey;
        self.g2_pubkey = *new_g2_pubkey;
        self.slot_registered = PodU64::from(current_slot);

        Ok(())
    }
}

impl Default for OperatorEntry {
    fn default() -> Self {
        Self::new(
            &Pubkey::default(),
            &Pubkey::default(),
            &[0; 32],
            &[0; 64],
            Self::EMPTY_OPERATOR_INDEX,
            Self::EMPTY_SLOT_REGISTERED,
            0,
        )
    }
}

impl fmt::Display for OperatorEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\n----------- Operator Entry -------------")?;
        writeln!(f, "  NCN:                          {}", self.ncn)?;
        writeln!(
            f,
            "  Operator:                     {}",
            self.operator_pubkey()
        )?;
        writeln!(f, "  G1 Pubkey:                    {:?}", self.g1_pubkey())?;
        writeln!(f, "  G2 Pubkey:                    {:?}", self.g2_pubkey())?;
        writeln!(
            f,
            "  Index:                        {}",
            self.operator_index()
        )?;
        writeln!(
            f,
            "  Slot Registered:              {}",
            self.slot_registered()
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::g1_point::G1CompressedPoint;
    use crate::g2_point::G2CompressedPoint;
    use crate::privkey::PrivKey;

    #[test]
    fn test_len() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<Pubkey>() // operator_pubkey
            + 32 // g1_pubkey
            + 64 // g2_pubkey
            + size_of::<PodU64>() // operator_index
            + size_of::<PodU64>() // slot_registered
            + 1 // bump
            + 199; // reserved

        assert_eq!(size_of::<OperatorEntry>(), expected_total);
    }

    #[test]
    #[cfg(not(target_os = "solana"))]
    fn test_create_operator_entry() {
        let ncn = Pubkey::new_unique();
        let operator = Pubkey::new_unique();

        // Generate valid keypair
        let private_key = PrivKey::from_random();
        let g1_compressed = G1CompressedPoint::try_from(private_key).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&private_key).unwrap();

        let operator_entry = OperatorEntry::new(
            &ncn,
            &operator,
            &g1_compressed.0,
            &g2_compressed.0,
            0,
            100,
            255,
        );

        assert_eq!(operator_entry.ncn(), &ncn);
        assert_eq!(operator_entry.operator_pubkey(), &operator);
        assert_eq!(operator_entry.g1_pubkey(), &g1_compressed.0);
        assert_eq!(operator_entry.g2_pubkey(), &g2_compressed.0);
        assert_eq!(operator_entry.operator_index(), 0);
        assert_eq!(operator_entry.slot_registered(), 100);
        assert_eq!(operator_entry.bump, 255);
    }

    #[test]
    #[cfg(not(target_os = "solana"))]
    fn test_keypair_verification() {
        let ncn = Pubkey::new_unique();
        let operator = Pubkey::new_unique();

        // Test valid keypair
        let private_key = PrivKey::from_random();
        let g1_compressed = G1CompressedPoint::try_from(private_key).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&private_key).unwrap();

        let valid_entry = OperatorEntry::new(
            &ncn,
            &operator,
            &g1_compressed.0,
            &g2_compressed.0,
            0,
            100,
            255,
        );

        assert!(valid_entry.verify_keypair().is_ok());

        // Test mismatched keypair
        let private_key2 = PrivKey::from_random();
        let g2_compressed_wrong = G2CompressedPoint::try_from(&private_key2).unwrap();

        let invalid_entry = OperatorEntry::new(
            &ncn,
            &operator,
            &g1_compressed.0,
            &g2_compressed_wrong.0,
            0,
            100,
            255,
        );

        assert!(invalid_entry.verify_keypair().is_err());
    }

    #[test]
    #[cfg(not(target_os = "solana"))]
    fn test_update_keys() {
        let ncn = Pubkey::new_unique();
        let operator = Pubkey::new_unique();

        // Initial valid keypair
        let private_key1 = PrivKey::from_random();
        let g1_compressed1 = G1CompressedPoint::try_from(private_key1).unwrap();
        let g2_compressed1 = G2CompressedPoint::try_from(&private_key1).unwrap();

        let mut operator_entry = OperatorEntry::new(
            &ncn,
            &operator,
            &g1_compressed1.0,
            &g2_compressed1.0,
            0,
            100,
            255,
        );

        // New valid keypair
        let private_key2 = PrivKey::from_random();
        let g1_compressed2 = G1CompressedPoint::try_from(private_key2).unwrap();
        let g2_compressed2 = G2CompressedPoint::try_from(&private_key2).unwrap();

        // Update should succeed
        assert!(operator_entry
            .update_keys(&g1_compressed2.0, &g2_compressed2.0, 200)
            .is_ok());

        assert_eq!(operator_entry.g1_pubkey(), &g1_compressed2.0);
        assert_eq!(operator_entry.g2_pubkey(), &g2_compressed2.0);
        assert_eq!(operator_entry.slot_registered(), 200);

        // Test update with mismatched keypair should fail
        let private_key3 = PrivKey::from_random();
        let g2_compressed_wrong = G2CompressedPoint::try_from(&private_key3).unwrap();

        assert!(operator_entry
            .update_keys(&g1_compressed2.0, &g2_compressed_wrong.0, 300)
            .is_err());
    }
}
