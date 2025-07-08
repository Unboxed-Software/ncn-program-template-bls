use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    constants::MAX_OPERATORS,
    discriminators::Discriminators,
    error::NCNProgramError,
    g1_point::{G1CompressedPoint, G1Point},
    g2_point::{G2CompressedPoint, G2Point},
    loaders::check_load,
};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct OperatorEntry {
    /// The operator pubkey
    operator_pubkey: Pubkey,
    /// The G1 pubkey in compressed format (32 bytes)
    g1_pubkey: [u8; 32],
    /// The G2 pubkey in compressed format (64 bytes)
    g2_pubkey: [u8; 64],
    /// The index of the operator in respect to the NCN account
    operator_index: PodU64,
    /// The slot the operator was registered
    slot_registered: PodU64,
}

impl OperatorEntry {
    pub const EMPTY_OPERATOR_INDEX: u64 = u64::MAX;
    pub const EMPTY_SLOT_REGISTERED: u64 = u64::MAX;

    pub fn new(
        operator_pubkey: &Pubkey,
        g1_pubkey: &[u8; 32],
        g2_pubkey: &[u8; 64],
        operator_index: u64,
        slot_registered: u64,
    ) -> Self {
        Self {
            operator_pubkey: *operator_pubkey,
            g1_pubkey: *g1_pubkey,
            g2_pubkey: *g2_pubkey,
            operator_index: PodU64::from(operator_index),
            slot_registered: PodU64::from(slot_registered),
        }
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
}

impl Default for OperatorEntry {
    fn default() -> Self {
        Self::new(
            &Pubkey::default(),
            &[0; 32],
            &[0; 64],
            Self::EMPTY_OPERATOR_INDEX,
            Self::EMPTY_SLOT_REGISTERED,
        )
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorRegistry {
    /// The NCN the operator registry is associated with
    pub ncn: Pubkey,
    /// The bump seed for the PDA
    pub bump: u8,
    /// The list of operators
    pub operator_list: [OperatorEntry; 256],
}

impl Discriminator for OperatorRegistry {
    const DISCRIMINATOR: u8 = Discriminators::OperatorRegistry as u8;
}

impl OperatorRegistry {
    const OPERATOR_REGISTRY_SEED: &'static [u8] = b"operator_registry";
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: &Pubkey, bump: u8) -> Self {
        Self {
            ncn: *ncn,
            bump,
            operator_list: [OperatorEntry::default(); MAX_OPERATORS],
        }
    }

    pub fn initialize(&mut self, ncn: &Pubkey, bump: u8) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = *ncn;
        self.bump = bump;
        self.operator_list = [OperatorEntry::default(); MAX_OPERATORS];
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                Self::OPERATOR_REGISTRY_SEED.to_vec(),
                ncn.to_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(program_id: &Pubkey, ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn has_operator(&self, operator: &Pubkey) -> bool {
        self.operator_list
            .iter()
            .any(|op| op.operator_pubkey.eq(operator))
    }

    pub fn register_operator(
        &mut self,
        operator_pubkey: &Pubkey,
        g1_pubkey: &[u8; 32],
        g2_pubkey: &[u8; 64],
        operator_index: u64,
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        // Check if operator is already in the list
        if self
            .operator_list
            .iter()
            .any(|op| op.operator_pubkey.eq(operator_pubkey))
        {
            return Ok(());
        }

        // Insert at the first empty slot
        let operator_entry = self
            .operator_list
            .iter_mut()
            .find(|op| op.operator_pubkey == OperatorEntry::default().operator_pubkey)
            .ok_or(NCNProgramError::OperatorRegistryListFull)?;

        let new_operator_entry = OperatorEntry::new(
            operator_pubkey,
            g1_pubkey,
            g2_pubkey,
            operator_index,
            current_slot,
        );

        // Verify the keypair before storing
        new_operator_entry.verify_keypair()?;

        *operator_entry = new_operator_entry;

        Ok(())
    }

    pub const fn get_operator_entries(&self) -> &[OperatorEntry; MAX_OPERATORS] {
        &self.operator_list
    }

    pub fn operator_count(&self) -> u64 {
        self.operator_list
            .iter()
            .filter(|op| !op.is_empty())
            .count() as u64
    }

    pub fn get_valid_operator_entries(&self) -> Vec<OperatorEntry> {
        self.operator_list
            .iter()
            .filter(|op| !op.is_empty())
            .copied()
            .collect()
    }

    pub fn get_operator_entry(
        &self,
        operator_pubkey: &Pubkey,
    ) -> Result<OperatorEntry, ProgramError> {
        let operator_entry = self
            .operator_list
            .iter()
            .find(|op| op.operator_pubkey().eq(operator_pubkey))
            .ok_or(NCNProgramError::OperatorEntryNotFound)?;

        Ok(*operator_entry)
    }

    pub fn update_operator_keys(
        &mut self,
        operator_pubkey: &Pubkey,
        new_g1_pubkey: &[u8; 32],
        new_g2_pubkey: &[u8; 64],
        current_slot: u64,
    ) -> Result<(), ProgramError> {
        // Find the operator entry
        let operator_entry = self
            .operator_list
            .iter_mut()
            .find(|op| op.operator_pubkey().eq(operator_pubkey))
            .ok_or(NCNProgramError::OperatorEntryNotFound)?;

        // Create a temporary entry with new keys to verify them
        let temp_entry = OperatorEntry::new(
            operator_pubkey,
            new_g1_pubkey,
            new_g2_pubkey,
            operator_entry.operator_index(),
            current_slot,
        );

        // Verify the new keypair before updating
        temp_entry.verify_keypair()?;

        // Update the keys
        operator_entry.g1_pubkey = *new_g1_pubkey;
        operator_entry.g2_pubkey = *new_g2_pubkey;
        operator_entry.slot_registered = PodU64::from(current_slot);

        Ok(())
    }
}

#[rustfmt::skip]
impl fmt::Display for OperatorRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\n----------- Operator Registry -------------")?;
        writeln!(f, "  NCN:                          {}", self.ncn)?;
        writeln!(f, "  Operators:                    ")?;
        for operator in self.get_valid_operator_entries() {
            writeln!(f, "    Operator:                   {}", operator.operator_pubkey())?;
            writeln!(f, "      G1 Pubkey:                {:?}", operator.g1_pubkey())?;
            writeln!(f, "      G2 Pubkey:                {:?}", operator.g2_pubkey())?;
            writeln!(f, "      Index:                    {}", operator.operator_index())?;
            writeln!(f, "      Slot Registered:          {}\n", operator.slot_registered())?;
        }

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
            + 1 // bump
            + size_of::<OperatorEntry>() * MAX_OPERATORS; // operator_list

        assert_eq!(size_of::<OperatorRegistry>(), expected_total);

        let operator_registry = OperatorRegistry::new(&Pubkey::default(), 0);
        assert_eq!(operator_registry.operator_list.len(), MAX_OPERATORS);
    }

    #[test]
    #[cfg(not(target_os = "solana"))]
    fn test_add_operator() {
        let mut operator_registry = OperatorRegistry::new(&Pubkey::default(), 0);
        let operator = Pubkey::new_unique();

        // Generate valid keypair
        let private_key = PrivKey::from_random();
        let g1_compressed = G1CompressedPoint::try_from(private_key).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&private_key).unwrap();

        // Test 1: Initial registration should succeed
        assert_eq!(operator_registry.get_valid_operator_entries().len(), 0);
        operator_registry
            .register_operator(&operator, &g1_compressed.0, &g2_compressed.0, 0, 100)
            .unwrap();
        assert_eq!(operator_registry.get_valid_operator_entries().len(), 1);

        // Test 2: Trying to add the same operator should succeed (no-op)
        operator_registry
            .register_operator(&operator, &g1_compressed.0, &g2_compressed.0, 0, 100)
            .unwrap();
        assert_eq!(operator_registry.get_valid_operator_entries().len(), 1);

        // Test 3: Adding a different operator should succeed
        let operator2 = Pubkey::new_unique();
        let private_key2 = PrivKey::from_random();
        let g1_compressed2 = G1CompressedPoint::try_from(private_key2).unwrap();
        let g2_compressed2 = G2CompressedPoint::try_from(&private_key2).unwrap();

        operator_registry
            .register_operator(&operator2, &g1_compressed2.0, &g2_compressed2.0, 1, 200)
            .unwrap();
        assert_eq!(operator_registry.get_valid_operator_entries().len(), 2);

        // Test 4: Verify operator entry data is stored correctly
        let entry = operator_registry.get_operator_entry(&operator).unwrap();
        assert_eq!(entry.operator_pubkey(), &operator);
        assert_eq!(entry.g1_pubkey(), &g1_compressed.0);
        assert_eq!(entry.g2_pubkey(), &g2_compressed.0);
        assert_eq!(entry.operator_index(), 0);
        assert_eq!(entry.slot_registered(), 100);

        // Test 5: has_operator should work correctly
        assert!(operator_registry.has_operator(&operator));
        assert!(operator_registry.has_operator(&operator2));
        assert!(!operator_registry.has_operator(&Pubkey::new_unique()));
    }

    #[test]
    fn test_operator_count() {
        let mut operator_registry = OperatorRegistry::new(&Pubkey::default(), 0);
        assert_eq!(operator_registry.operator_count(), 0);

        // Add some dummy entries (without verification)
        for i in 0..3 {
            let entry = OperatorEntry::new(&Pubkey::new_unique(), &[0; 32], &[0; 64], i, 100);
            operator_registry.operator_list[i as usize] = entry;
        }
        assert_eq!(operator_registry.operator_count(), 3);
    }

    #[test]
    #[cfg(not(target_os = "solana"))]
    fn test_keypair_verification() {
        // Test valid keypair
        let private_key = PrivKey::from_random();
        let g1_compressed = G1CompressedPoint::try_from(private_key).unwrap();
        let g2_compressed = G2CompressedPoint::try_from(&private_key).unwrap();

        let valid_entry = OperatorEntry::new(
            &Pubkey::new_unique(),
            &g1_compressed.0,
            &g2_compressed.0,
            0,
            100,
        );

        assert!(valid_entry.verify_keypair().is_ok());

        // Test mismatched keypair
        let private_key2 = PrivKey::from_random();
        let g2_compressed_wrong = G2CompressedPoint::try_from(&private_key2).unwrap();

        let invalid_entry = OperatorEntry::new(
            &Pubkey::new_unique(),
            &g1_compressed.0,
            &g2_compressed_wrong.0,
            0,
            100,
        );

        assert!(invalid_entry.verify_keypair().is_err());
    }
}
