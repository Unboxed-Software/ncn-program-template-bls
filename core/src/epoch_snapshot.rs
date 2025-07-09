use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use jito_vault_core::vault_operator_delegation::VaultOperatorDelegation;
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::{G1_COMPRESSED_POINT_SIZE, MAX_VAULTS},
    discriminators::Discriminators,
    error::NCNProgramError,
    fees::Fees,
    g1_point::{G1CompressedPoint, G1Point},
    loaders::check_load,
    stake_weight::StakeWeights,
    weight_table::WeightTable,
};

// PDA'd ["epoch_snapshot", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochSnapshot {
    /// The NCN this snapshot is for
    ncn: Pubkey,
    /// The epoch this snapshot is for
    epoch: PodU64,
    /// Bump seed for the PDA
    bump: u8,
    /// Slot Epoch snapshot was created
    slot_created: PodU64,
    /// Slot Epoch snapshot was finalized
    slot_finalized: PodU64,
    /// Number of operators in the epoch
    operator_count: PodU64,
    /// Number of vaults in the epoch
    vault_count: PodU64,
    /// Keeps track of the number of completed operator registration through `snapshot_vault_operator_delegation` and `initialize_operator_snapshot`
    operators_registered: PodU64,
    /// Keeps track of the number of valid operator vault delegations
    valid_operator_vault_delegations: PodU64,
    /// Tallies the total stake weights for all vault operator delegations
    stake_weights: StakeWeights,
    /// total Operators G1 Pubkey aggregated stake weights
    total_agg_g1_pubkey: [u8; 32],
    /// Snapshot of the fees configuration for this epoch
    fees: Fees,
}

impl Discriminator for EpochSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochSnapshot {
    const EPOCH_SNAPSHOT_SEED: &'static [u8] = b"epoch_snapshot";
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(
        ncn: &Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        operator_count: u64,
        vault_count: u64,
        fees: Fees,
    ) -> Self {
        Self {
            ncn: *ncn,
            epoch: PodU64::from(ncn_epoch),
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            bump,
            operator_count: PodU64::from(operator_count),
            vault_count: PodU64::from(vault_count),
            operators_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            stake_weights: StakeWeights::default(),
            total_agg_g1_pubkey: [0; G1_COMPRESSED_POINT_SIZE],
            fees,
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                Self::EPOCH_SNAPSHOT_SEED.to_vec(),
                ncn.to_bytes().to_vec(),
                ncn_epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn, epoch).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn load_to_close(
        program_id: &Pubkey,
        account_to_close: &AccountInfo,
        ncn: &Pubkey,
        epoch: u64,
    ) -> Result<(), ProgramError> {
        Self::load(program_id, account_to_close, ncn, epoch, true)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn operator_count(&self) -> u64 {
        self.operator_count.into()
    }

    pub fn vault_count(&self) -> u64 {
        self.vault_count.into()
    }

    pub fn operators_registered(&self) -> u64 {
        self.operators_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weights
    }

    pub const fn total_agg_g1_pubkey(&self) -> &[u8; G1_COMPRESSED_POINT_SIZE] {
        &self.total_agg_g1_pubkey
    }

    pub fn slot_finalized(&self) -> u64 {
        self.slot_finalized.into()
    }

    pub fn finalized(&self) -> bool {
        self.operators_registered() == self.operator_count()
    }

    pub fn increment_operator_registration(
        &mut self,
        current_slot: u64,
        vault_operator_delegations: u64,
        stake_weight: &StakeWeights,
    ) -> Result<(), NCNProgramError> {
        if self.finalized() {
            return Err(NCNProgramError::OperatorFinalized);
        }

        self.operators_registered = PodU64::from(
            self.operators_registered()
                .checked_add(1)
                .ok_or(NCNProgramError::ArithmeticOverflow)?,
        );

        self.valid_operator_vault_delegations = PodU64::from(
            self.valid_operator_vault_delegations()
                .checked_add(vault_operator_delegations)
                .ok_or(NCNProgramError::ArithmeticOverflow)?,
        );

        self.stake_weights.increment(stake_weight)?;

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn register_operator_g1_pubkey(
        &mut self,
        operator_g1_pubkey: &[u8; G1_COMPRESSED_POINT_SIZE],
    ) -> Result<(), NCNProgramError> {
        // If the current aggregated pubkey is all zeros, replace it with the operator's pubkey
        if self.total_agg_g1_pubkey == [0u8; G1_COMPRESSED_POINT_SIZE] {
            self.total_agg_g1_pubkey = *operator_g1_pubkey;
        } else {
            // Otherwise, add them together
            let total_agg_g1_pubkey_point =
                G1Point::try_from(&G1CompressedPoint(self.total_agg_g1_pubkey))?;
            let operator_g1_pubkey_point =
                G1Point::try_from(&G1CompressedPoint(*operator_g1_pubkey))?;
            let new_total_agg_g1_pubkey_point =
                total_agg_g1_pubkey_point + operator_g1_pubkey_point;
            let compressed_point = G1CompressedPoint::try_from(new_total_agg_g1_pubkey_point)?;

            self.total_agg_g1_pubkey = compressed_point.0;
        }

        Ok(())
    }

    pub const fn fees(&self) -> &Fees {
        &self.fees
    }
}

// PDA'd ["operator_snapshot", OPERATOR, NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorSnapshot {
    operator: Pubkey,
    g1_pubkey: [u8; 32], // G1 compressed pubkey

    ncn: Pubkey,
    ncn_epoch: PodU64,
    bump: u8,

    slot_created: PodU64,
    slot_finalized: PodU64,

    is_active: PodBool,

    ncn_operator_index: PodU64,
    operator_index: PodU64,
    operator_fee_bps: PodU16,

    vault_operator_delegation_count: PodU64,
    vault_operator_delegations_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    stake_weights: StakeWeights,

    vault_operator_stake_weight: [VaultOperatorStakeWeight; 64],
}

impl Discriminator for OperatorSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::OperatorSnapshot as u8;
}

impl OperatorSnapshot {
    const OPERATOR_SNAPSHOT_SEED: &'static [u8] = b"operator_snapshot";
    pub const SIZE: usize = 8 + size_of::<Self>();

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        is_active: bool,
        ncn_operator_index: u64,
        operator_index: u64,
        operator_fee_bps: u16,
        vault_operator_delegation_count: u64,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
    ) -> Result<Self, NCNProgramError> {
        if vault_operator_delegation_count > MAX_VAULTS as u64 {
            return Err(NCNProgramError::TooManyVaultOperatorDelegations);
        }

        Ok(Self {
            operator: *operator,
            ncn: *ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            is_active: PodBool::from(is_active),
            ncn_operator_index: PodU64::from(ncn_operator_index),
            operator_index: PodU64::from(operator_index),
            operator_fee_bps: PodU16::from(operator_fee_bps),
            vault_operator_delegation_count: PodU64::from(vault_operator_delegation_count),
            vault_operator_delegations_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
            stake_weights: StakeWeights::default(),
            vault_operator_stake_weight: [VaultOperatorStakeWeight::default(); MAX_VAULTS],
            g1_pubkey,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        operator: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        is_active: bool,
        ncn_operator_index: u64,
        operator_index: u64,
        operator_fee_bps: u16,
        vault_operator_delegation_count: u64,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
    ) -> Result<(), NCNProgramError> {
        if vault_operator_delegation_count > MAX_VAULTS as u64 {
            return Err(NCNProgramError::TooManyVaultOperatorDelegations);
        }
        let slot_finalized = if !is_active { current_slot } else { 0 };
        let operator_fee_bps = if is_active { operator_fee_bps } else { 0 };
        let vault_operator_delegation_count = if is_active {
            vault_operator_delegation_count
        } else {
            0
        };

        // Initializes field by field to avoid overflowing stack
        self.operator = *operator;
        self.ncn = *ncn;
        self.ncn_epoch = PodU64::from(ncn_epoch);
        self.bump = bump;
        self.slot_created = PodU64::from(current_slot);
        self.slot_finalized = PodU64::from(slot_finalized);
        self.is_active = PodBool::from(is_active);
        self.ncn_operator_index = PodU64::from(ncn_operator_index);
        self.operator_index = PodU64::from(operator_index);
        self.operator_fee_bps = PodU16::from(operator_fee_bps);
        self.vault_operator_delegation_count = PodU64::from(vault_operator_delegation_count);
        self.vault_operator_delegations_registered = PodU64::from(0);
        self.valid_operator_vault_delegations = PodU64::from(0);
        self.stake_weights = StakeWeights::default();
        self.vault_operator_stake_weight = [VaultOperatorStakeWeight::default(); MAX_VAULTS];
        self.g1_pubkey = g1_pubkey;

        Ok(())
    }

    pub fn seeds(operator: &Pubkey, ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                Self::OPERATOR_SNAPSHOT_SEED.to_vec(),
                operator.to_bytes().to_vec(),
                ncn.to_bytes().to_vec(),
                ncn_epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        operator: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(operator, ncn, epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        operator: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, operator, ncn, epoch).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn load_to_close(
        program_id: &Pubkey,
        account_to_close: &AccountInfo,
        ncn: &Pubkey,
        epoch: u64,
    ) -> Result<(), ProgramError> {
        let account_data = account_to_close.try_borrow_data()?;
        let account_struct = Self::try_from_slice_unchecked(&account_data)?;
        let operator = *account_struct.operator();

        Self::load(program_id, account_to_close, &operator, ncn, epoch, true)
    }

    pub fn epoch(&self) -> u64 {
        self.ncn_epoch.into()
    }

    pub fn ncn_operator_index(&self) -> u64 {
        self.ncn_operator_index.into()
    }

    pub fn slot_finalized(&self) -> u64 {
        self.slot_finalized.into()
    }

    pub fn operator_fee_bps(&self) -> u16 {
        self.operator_fee_bps.into()
    }

    pub fn is_active(&self) -> bool {
        self.is_active.into()
    }

    pub fn g1_pubkey(&self) -> [u8; G1_COMPRESSED_POINT_SIZE] {
        self.g1_pubkey
    }

    pub fn have_valid_bn128_g1_pubkey(&self) -> bool {
        G1CompressedPoint::try_from(self.g1_pubkey).is_ok()
            && self.g1_pubkey != G1CompressedPoint::default().0
    }

    pub const fn operator(&self) -> &Pubkey {
        &self.operator
    }

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }

    pub fn vault_operator_delegation_count(&self) -> u64 {
        self.vault_operator_delegation_count.into()
    }

    pub fn vault_operator_delegations_registered(&self) -> u64 {
        self.vault_operator_delegations_registered.into()
    }

    pub fn valid_operator_vault_delegations(&self) -> u64 {
        self.valid_operator_vault_delegations.into()
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weights
    }

    pub fn finalized(&self) -> bool {
        self.vault_operator_delegations_registered() == self.vault_operator_delegation_count()
    }

    pub fn contains_vault_index(&self, vault_index: u64) -> bool {
        self.vault_operator_stake_weight
            .iter()
            .any(|v| v.vault_index() == vault_index)
    }

    pub fn contains_vault(&self, vault: &Pubkey) -> bool {
        self.vault_operator_stake_weight
            .iter()
            .any(|v| v.vault().eq(vault))
    }

    pub const fn vault_operator_stake_weight(&self) -> &[VaultOperatorStakeWeight] {
        &self.vault_operator_stake_weight
    }

    pub fn insert_vault_operator_stake_weight(
        &mut self,
        vault: &Pubkey,
        vault_index: u64,
        stake_weights: &StakeWeights,
    ) -> Result<(), NCNProgramError> {
        if self
            .vault_operator_delegations_registered()
            .checked_add(1)
            .ok_or(NCNProgramError::ArithmeticOverflow)?
            > MAX_VAULTS as u64
        {
            return Err(NCNProgramError::TooManyVaultOperatorDelegations);
        }

        if self.contains_vault_index(vault_index) {
            return Err(NCNProgramError::DuplicateVaultOperatorDelegation);
        }

        self.vault_operator_stake_weight[self.vault_operator_delegations_registered() as usize] =
            VaultOperatorStakeWeight::new(vault, vault_index, stake_weights);

        Ok(())
    }

    pub fn increment_vault_operator_delegation_registration(
        &mut self,
        current_slot: u64,
        vault: &Pubkey,
        vault_index: u64,
        stake_weights: &StakeWeights,
    ) -> Result<(), NCNProgramError> {
        if self.finalized() {
            return Err(NCNProgramError::VaultOperatorDelegationFinalized);
        }

        self.insert_vault_operator_stake_weight(vault, vault_index, stake_weights)?;

        self.vault_operator_delegations_registered = PodU64::from(
            self.vault_operator_delegations_registered()
                .checked_add(1)
                .ok_or(NCNProgramError::ArithmeticOverflow)?,
        );

        if stake_weights.stake_weight() > 0 {
            self.valid_operator_vault_delegations = PodU64::from(
                self.valid_operator_vault_delegations()
                    .checked_add(1)
                    .ok_or(NCNProgramError::ArithmeticOverflow)?,
            );
        }

        self.stake_weights.increment(stake_weights)?;

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn calculate_total_stake_weight(
        vault_operator_delegation: &VaultOperatorDelegation,
        weight_table: &WeightTable,
        st_mint: &Pubkey,
    ) -> Result<u128, ProgramError> {
        let total_security = vault_operator_delegation
            .delegation_state
            .total_security()?;

        let precise_total_security = PreciseNumber::new(total_security as u128)
            .ok_or(NCNProgramError::NewPreciseNumberError)?;

        let precise_weight = weight_table.get_precise_weight(st_mint)?;

        let precise_total_stake_weight = precise_total_security
            .checked_mul(&precise_weight)
            .ok_or(NCNProgramError::ArithmeticOverflow)?;

        let total_stake_weight = precise_total_stake_weight
            .to_imprecise()
            .ok_or(NCNProgramError::CastToImpreciseNumberError)?;

        Ok(total_stake_weight)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, ShankType)]
#[repr(C)]
pub struct VaultOperatorStakeWeight {
    vault: Pubkey,
    vault_index: PodU64,
    stake_weight: StakeWeights,
}

impl Default for VaultOperatorStakeWeight {
    fn default() -> Self {
        Self {
            vault: Pubkey::default(),
            vault_index: PodU64::from(u64::MAX),
            stake_weight: StakeWeights::default(),
        }
    }
}

impl VaultOperatorStakeWeight {
    pub fn new(vault: &Pubkey, vault_index: u64, stake_weight: &StakeWeights) -> Self {
        Self {
            vault: *vault,
            vault_index: PodU64::from(vault_index),
            stake_weight: *stake_weight,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.vault_index() == u64::MAX
    }

    pub fn vault_index(&self) -> u64 {
        self.vault_index.into()
    }

    pub const fn stake_weights(&self) -> &StakeWeights {
        &self.stake_weight
    }

    pub const fn vault(&self) -> &Pubkey {
        &self.vault
    }
}

#[rustfmt::skip]
impl fmt::Display for EpochSnapshot {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       writeln!(f, "\n\n----------- Epoch Snapshot -------------")?;
       writeln!(f, "  NCN:                          {}", self.ncn)?;
       writeln!(f, "  Epoch:                        {}", self.epoch())?;
       writeln!(f, "  Bump:                         {}", self.bump)?;
       writeln!(f, "  Operator Count:               {}", self.operator_count())?;
       writeln!(f, "  Vault Count:                  {}", self.vault_count())?;
       writeln!(f, "  Operators Registered:         {}", self.operators_registered())?;
       writeln!(f, "  Valid Delegations:            {}", self.valid_operator_vault_delegations())?;
       writeln!(f, "  Slot Finalized:               {}", self.slot_finalized())?;
       writeln!(f, "  Finalized:                    {}", self.finalized())?;
       writeln!(f, "  total Weight:                 {}", self.stake_weights().stake_weight())?;
       writeln!(f, "  Protocol Fee BPS:             {}", self.fees().protocol_fee_bps().unwrap_or(0))?;
       writeln!(f, "  NCN Fee BPS:                  {}", self.fees().ncn_fee_bps().unwrap_or(0))?;
       writeln!(f, "  Total Fee BPS:                {}", self.fees().total_fees_bps().unwrap_or(0))?;
       writeln!(f, "  Total Agg G1 Pubkey:          {:?}", self.total_agg_g1_pubkey())?;

       writeln!(f, "\n")?;
       Ok(())
   }
}

#[rustfmt::skip]
impl fmt::Display for OperatorSnapshot {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       writeln!(f, "\n\n----------- Operator Snapshot -------------")?;
       writeln!(f, "  Operator:                     {}", self.operator)?;
       writeln!(f, "  NCN:                          {}", self.ncn)?;
       writeln!(f, "  Epoch:                        {}", self.epoch())?;
       writeln!(f, "  Bump:                         {}", self.bump)?;
       writeln!(f, "  Slot Finalized:               {}", self.slot_finalized())?;
       writeln!(f, "  Is Active:                    {}", self.is_active())?;
       writeln!(f, "  NCN Operator Index:           {}", self.ncn_operator_index())?;
       writeln!(f, "  Operator Fee BPS:             {}", self.operator_fee_bps())?;
       writeln!(f, "  Delegation Count:             {}", self.vault_operator_delegation_count())?;
       writeln!(f, "  Delegations Registered:       {}", self.vault_operator_delegations_registered())?;
       writeln!(f, "  Valid Delegations:            {}", self.valid_operator_vault_delegations())?;
       writeln!(f, "  Finalized:                    {}", self.finalized())?;
       writeln!(f, "  G1 Pubkey:                    {:?}", self.g1_pubkey())?;

       let stake_weights = self.stake_weights();
       writeln!(f, "\nTotal Stake Weight: {}", stake_weights.stake_weight())?;

       writeln!(f, "\nVault Operator Stake Weights:")?;
       for weight in self.vault_operator_stake_weight().iter() {
           if !weight.is_empty() {
               writeln!(f, "  Vault:                        {}", weight.vault())?;
               writeln!(f, "    Vault Index:                {}", weight.vault_index())?;
               writeln!(f, "    Stake Weight: {}", weight.stake_weights().stake_weight())?;
           }
       }

       writeln!(f, "\n")?;
       Ok(())
   }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_snapshot_size() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // operator
            + size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // ncn_epoch
            + 1 // bump
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // slot_finalized
            + size_of::<PodBool>() // is_active
            + size_of::<PodU64>() // ncn_operator_index
            + size_of::<PodU64>() // operator_index
            + size_of::<PodU16>() // operator_fee_bps
            + size_of::<PodU64>() // vault_operator_delegation_count
            + size_of::<PodU64>() // vault_operator_delegations_registered
            + size_of::<PodU64>() // valid_operator_vault_delegations
            + size_of::<StakeWeights>() // stake_weight
            + size_of::<VaultOperatorStakeWeight>() * MAX_VAULTS // vault_operator_stake_weight
            + size_of::<[u8; G1_COMPRESSED_POINT_SIZE]>(); // g1_pubkey

        assert_eq!(size_of::<OperatorSnapshot>(), expected_total);
    }

    #[test]
    fn test_vault_operator_stake_weight_is_empty() {
        // Test default (should be empty)
        let default_weight = VaultOperatorStakeWeight::default();
        assert!(default_weight.is_empty());

        // Test non-empty case
        let non_empty_weight =
            VaultOperatorStakeWeight::new(&Pubkey::new_unique(), 1, &StakeWeights::default());
        assert!(!non_empty_weight.is_empty());
    }

    #[test]
    fn test_increment_vault_operator_delegation_registration_finalized() {
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let mut snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,
            1,
            100,
            true,
            0,
            0,
            100,
            1, // Set vault_operator_delegation_count to 1
            g1_pubkey,
        )
        .unwrap();

        // Register one delegation to reach finalized state
        snapshot.vault_operator_delegations_registered = PodU64::from(1);

        // Attempt to increment when finalized
        let result = snapshot.increment_vault_operator_delegation_registration(
            200, // current_slot
            &Pubkey::new_unique(),
            1,
            &StakeWeights::default(),
        );

        // Verify we get the expected error
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::VaultOperatorDelegationFinalized
        );
    }

    #[test]
    fn test_initialize_too_many_vault_operator_delegations() {
        // Create an operator snapshot
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let mut snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,
            1,
            100,
            true,
            0,
            0,
            100,
            1, // Set vault_operator_delegation_count to 1
            g1_pubkey,
        )
        .unwrap();

        // Try to initialize with vault_operator_delegation_count > MAX_VAULTS
        let result = snapshot.initialize(
            &Pubkey::new_unique(),   // operator
            &Pubkey::new_unique(),   // ncn
            1,                       // ncn_epoch
            1,                       // bump
            100,                     // current_slot
            true,                    // is_active
            0,                       // ncn_operator_index
            0,                       // operator_index
            100,                     // operator_fee_bps
            (MAX_VAULTS as u64) + 1, // vault_operator_delegation_count > MAX_VAULTS
            g1_pubkey,
        );

        // Verify we get the expected error
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::TooManyVaultOperatorDelegations
        );
    }

    #[test]
    fn test_insert_vault_operator_stake_weight_too_many_delegations() {
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let mut snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,
            1,
            100,
            true,
            0,
            0,
            100,
            1,
            g1_pubkey,
        )
        .unwrap();

        // Set the registered delegations to MAX_VAULTS
        snapshot.vault_operator_delegations_registered = PodU64::from(MAX_VAULTS as u64);

        // Try to insert another vault operator stake weight
        let result = snapshot.insert_vault_operator_stake_weight(
            &Pubkey::new_unique(),
            1,
            &StakeWeights::default(),
        );

        // Verify we get the expected error
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::TooManyVaultOperatorDelegations
        );
    }

    #[test]
    fn test_insert_vault_operator_stake_weight_duplicate_delegation() {
        // Create an operator snapshot
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let mut snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,
            1,
            100,
            true,
            0,
            0,
            100,
            2, // Allow for 2 delegations
            g1_pubkey,
        )
        .unwrap();

        let vault_index = 42;

        // Insert first vault operator stake weight
        snapshot
            .insert_vault_operator_stake_weight(
                &Pubkey::new_unique(),
                vault_index, // Use specific index
                &StakeWeights::default(),
            )
            .unwrap();

        // Increment the registered count as would happen in normal operation
        snapshot.vault_operator_delegations_registered = PodU64::from(1);

        // Try to insert another vault operator stake weight with the same index
        let result = snapshot.insert_vault_operator_stake_weight(
            &Pubkey::new_unique(),
            vault_index, // Use same index as before
            &StakeWeights::default(),
        );

        // Verify we get the expected error
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::DuplicateVaultOperatorDelegation
        );
    }

    #[test]
    fn test_operator_snapshot_new_too_many_delegations() {
        // Try to create a new OperatorSnapshot with vault_operator_delegation_count > MAX_VAULTS
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let result = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,                       // ncn_epoch
            1,                       // bump
            100,                     // current_slot
            true,                    // is_active
            0,                       // ncn_operator_index
            0,                       // operator_index
            100,                     // operator_fee_bps
            (MAX_VAULTS as u64) + 1, // vault_operator_delegation_count exceeds MAX_VAULTS
            g1_pubkey,
        );

        // Verify we get the expected error
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::TooManyVaultOperatorDelegations
        );
    }

    #[test]
    fn test_increment_operator_registration_finalized() {
        // Create an epoch snapshot
        let mut snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                          // ncn_epoch
            1,                          // bump
            100,                        // current_slot
            1,                          // operator_count - set to 1
            1,                          // vault_count
            Fees::new(100, 1).unwrap(), // fees
        );

        // Set operators_registered equal to operator_count to make it finalized
        snapshot.operators_registered = PodU64::from(1);

        // Try to increment operator registration when already finalized
        let result = snapshot.increment_operator_registration(
            200, // current_slot
            1,   // vault_operator_delegations
            &StakeWeights::default(),
        );

        // Verify we get the expected error
        assert_eq!(result.unwrap_err(), NCNProgramError::OperatorFinalized);
    }

    #[test]
    fn test_operator_snapshot_initialize_active_inactive() {
        let current_slot = 100;
        let operator_fee_bps = 150;
        let vault_operator_delegation_count = 3;
        let g1_pubkey = G1CompressedPoint::from_random().0;

        // Create two operator snapshots - one for active and one for inactive
        let mut active_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,
            1,
            current_slot,
            true,
            0,
            0,
            100,
            1,
            g1_pubkey,
        )
        .unwrap();

        let mut inactive_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,
            1,
            current_slot,
            false,
            0,
            0,
            100,
            1,
            g1_pubkey,
        )
        .unwrap();

        // Initialize active snapshot
        active_snapshot
            .initialize(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                1,
                1,
                current_slot,
                true, // is_active
                0,
                0,
                operator_fee_bps,
                vault_operator_delegation_count,
                g1_pubkey,
            )
            .unwrap();

        // Initialize inactive snapshot
        inactive_snapshot
            .initialize(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                1,
                1,
                current_slot,
                false, // not active
                0,
                0,
                operator_fee_bps,
                vault_operator_delegation_count,
                g1_pubkey,
            )
            .unwrap();

        // Test active snapshot values
        assert_eq!(active_snapshot.slot_finalized(), 0); // slot_finalized should be 0
        assert_eq!(active_snapshot.operator_fee_bps(), operator_fee_bps); // should keep original fee
        assert_eq!(
            active_snapshot.vault_operator_delegation_count(),
            vault_operator_delegation_count
        ); // should keep original count

        // Test inactive snapshot values
        assert_eq!(inactive_snapshot.slot_finalized(), current_slot); // slot_finalized should be current_slot
        assert_eq!(inactive_snapshot.operator_fee_bps(), 0); // fee should be zeroed
        assert_eq!(inactive_snapshot.vault_operator_delegation_count(), 0);
        // count should be zeroed
    }

    #[test]
    fn test_register_operator_g1_pubkey() {
        // Create an epoch snapshot with default aggregated G1 pubkey (all zeros)
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                          // ncn_epoch
            1,                          // bump
            100,                        // current_slot
            3,                          // operator_count
            1,                          // vault_count
            Fees::new(100, 1).unwrap(), // fees
        );

        // Verify initial state - total_agg_g1_pubkey should be all zeros
        assert_eq!(
            epoch_snapshot.total_agg_g1_pubkey(),
            &[0u8; G1_COMPRESSED_POINT_SIZE]
        );

        // Generate a random G1 pubkey
        let operator_g1_pubkey = G1CompressedPoint::from_random().0;

        // Register the operator's G1 pubkey
        let result = epoch_snapshot.register_operator_g1_pubkey(&operator_g1_pubkey);
        assert!(result.is_ok());

        // Verify the aggregated G1 pubkey is no longer all zeros
        assert_ne!(
            epoch_snapshot.total_agg_g1_pubkey(),
            &[0u8; G1_COMPRESSED_POINT_SIZE]
        );

        // The aggregated pubkey should now equal the first operator's pubkey
        // since we started with zero point (identity element for addition)
        assert_eq!(epoch_snapshot.total_agg_g1_pubkey(), &operator_g1_pubkey);
    }

    #[test]
    fn test_register_multiple_operator_g1_pubkeys() {
        // Create an epoch snapshot
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                          // ncn_epoch
            1,                          // bump
            100,                        // current_slot
            3,                          // operator_count
            1,                          // vault_count
            Fees::new(100, 1).unwrap(), // fees
        );

        // Generate multiple random G1 pubkeys
        let operator1_g1_pubkey = G1CompressedPoint::from_random().0;
        let operator2_g1_pubkey = G1CompressedPoint::from_random().0;
        let operator3_g1_pubkey = G1CompressedPoint::from_random().0;

        // Register first operator's G1 pubkey
        epoch_snapshot
            .register_operator_g1_pubkey(&operator1_g1_pubkey)
            .unwrap();
        let after_first = *epoch_snapshot.total_agg_g1_pubkey();

        // Register second operator's G1 pubkey
        epoch_snapshot
            .register_operator_g1_pubkey(&operator2_g1_pubkey)
            .unwrap();
        let after_second = *epoch_snapshot.total_agg_g1_pubkey();

        // Register third operator's G1 pubkey
        epoch_snapshot
            .register_operator_g1_pubkey(&operator3_g1_pubkey)
            .unwrap();
        let after_third = *epoch_snapshot.total_agg_g1_pubkey();

        // Verify that each registration changes the aggregated pubkey
        assert_ne!(after_first, [0u8; G1_COMPRESSED_POINT_SIZE]);
        assert_ne!(after_second, after_first);
        assert_ne!(after_third, after_second);

        // Manually compute the expected aggregated result to verify correctness
        let point1 = G1Point::try_from(&G1CompressedPoint(operator1_g1_pubkey)).unwrap();
        let point2 = G1Point::try_from(&G1CompressedPoint(operator2_g1_pubkey)).unwrap();
        let point3 = G1Point::try_from(&G1CompressedPoint(operator3_g1_pubkey)).unwrap();

        let expected_aggregate = point1 + point2 + point3;
        let expected_compressed = G1CompressedPoint::try_from(expected_aggregate).unwrap();

        assert_eq!(epoch_snapshot.total_agg_g1_pubkey(), &expected_compressed.0);
    }

    #[test]
    fn test_operator_snapshot_g1_pubkey_storage() {
        // Generate a random G1 pubkey
        let g1_pubkey = G1CompressedPoint::from_random().0;

        // Create an operator snapshot with the G1 pubkey
        let operator_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,         // ncn_epoch
            1,         // bump
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            100,       // operator_fee_bps
            1,         // vault_operator_delegation_count
            g1_pubkey, // g1_pubkey
        )
        .unwrap();

        // Verify the G1 pubkey is stored correctly
        assert_eq!(operator_snapshot.g1_pubkey, g1_pubkey);
    }

    #[test]
    fn test_operator_snapshot_initialize_g1_pubkey() {
        // Generate two different G1 pubkeys
        let original_g1_pubkey = G1CompressedPoint::from_random().0;
        let new_g1_pubkey = G1CompressedPoint::from_random().0;

        // Create an operator snapshot with the original G1 pubkey
        let mut operator_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            &Pubkey::new_unique(),
            1,                  // ncn_epoch
            1,                  // bump
            100,                // current_slot
            true,               // is_active
            0,                  // ncn_operator_index
            0,                  // operator_index
            100,                // operator_fee_bps
            1,                  // vault_operator_delegation_count
            original_g1_pubkey, // original g1_pubkey
        )
        .unwrap();

        // Initialize with a new G1 pubkey
        operator_snapshot
            .initialize(
                &Pubkey::new_unique(),
                &Pubkey::new_unique(),
                1,             // ncn_epoch
                1,             // bump
                100,           // current_slot
                true,          // is_active
                0,             // ncn_operator_index
                0,             // operator_index
                100,           // operator_fee_bps
                1,             // vault_operator_delegation_count
                new_g1_pubkey, // new g1_pubkey
            )
            .unwrap();

        // Verify the G1 pubkey was updated during initialization
        assert_eq!(operator_snapshot.g1_pubkey, new_g1_pubkey);
        assert_ne!(operator_snapshot.g1_pubkey, original_g1_pubkey);
    }

    #[test]
    fn test_epoch_snapshot_aggregation_order_independence() {
        // Test that G1 pubkey aggregation is order-independent (commutative)
        let operator1_g1_pubkey = G1CompressedPoint::from_random().0;
        let operator2_g1_pubkey = G1CompressedPoint::from_random().0;

        // Create two epoch snapshots
        let mut snapshot1 = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                          // ncn_epoch
            1,                          // bump
            100,                        // current_slot
            2,                          // operator_count
            1,                          // vault_count
            Fees::new(100, 1).unwrap(), // fees
        );

        let mut snapshot2 = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                          // ncn_epoch
            1,                          // bump
            100,                        // current_slot
            2,                          // operator_count
            1,                          // vault_count
            Fees::new(100, 1).unwrap(), // fees
        );

        // Register operators in different orders
        // Snapshot 1: operator1 first, then operator2
        snapshot1
            .register_operator_g1_pubkey(&operator1_g1_pubkey)
            .unwrap();
        snapshot1
            .register_operator_g1_pubkey(&operator2_g1_pubkey)
            .unwrap();

        // Snapshot 2: operator2 first, then operator1
        snapshot2
            .register_operator_g1_pubkey(&operator2_g1_pubkey)
            .unwrap();
        snapshot2
            .register_operator_g1_pubkey(&operator1_g1_pubkey)
            .unwrap();

        // Both should result in the same aggregated G1 pubkey
        assert_eq!(
            snapshot1.total_agg_g1_pubkey(),
            snapshot2.total_agg_g1_pubkey()
        );
    }

    #[test]
    fn test_epoch_snapshot_g1_pubkey_getter() {
        // Create an epoch snapshot
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                          // ncn_epoch
            1,                          // bump
            100,                        // current_slot
            1,                          // operator_count
            1,                          // vault_count
            Fees::new(100, 1).unwrap(), // fees
        );

        // Initially should be all zeros
        assert_eq!(
            epoch_snapshot.total_agg_g1_pubkey(),
            &[0u8; G1_COMPRESSED_POINT_SIZE]
        );

        // Register an operator G1 pubkey
        let g1_pubkey = G1CompressedPoint::from_random().0;
        epoch_snapshot
            .register_operator_g1_pubkey(&g1_pubkey)
            .unwrap();

        // Verify getter returns the updated value
        assert_eq!(epoch_snapshot.total_agg_g1_pubkey(), &g1_pubkey);
    }
}
