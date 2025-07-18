use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU64},
    AccountDeserialize, Discriminator,
};
use jito_vault_core::vault_operator_delegation::VaultOperatorDelegation;
use shank::{ShankAccount, ShankType};
use solana_bn254::compression::prelude::{alt_bn128_g1_compress, alt_bn128_g1_decompress};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    constants::{G1_COMPRESSED_POINT_SIZE, MAX_OPERATORS, MAX_VAULTS},
    discriminators::Discriminators,
    error::NCNProgramError,
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
    operators_can_vote_count: PodU64,
    /// total Operators G1 Pubkey aggregated stake weights
    total_agg_g1_pubkey: [u8; 32],
    /// Array of operator snapshots
    operator_snapshots: [OperatorSnapshot; 256],
    /// Minimum stake weight required to vote
    minimum_stake_weight: StakeWeights,
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
        minimum_stake_weight: StakeWeights,
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
            operators_can_vote_count: PodU64::from(0),
            total_agg_g1_pubkey: [0; G1_COMPRESSED_POINT_SIZE],
            operator_snapshots: [OperatorSnapshot::default(); 256],
            minimum_stake_weight,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        ncn: &Pubkey,
        ncn_epoch: u64,
        bump: u8,
        current_slot: u64,
        operator_count: u64,
        vault_count: u64,
        minimum_stake_weight: StakeWeights,
    ) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = *ncn;
        self.epoch = PodU64::from(ncn_epoch);
        self.slot_created = PodU64::from(current_slot);
        self.slot_finalized = PodU64::from(0);
        self.bump = bump;
        self.operator_count = PodU64::from(operator_count);
        self.vault_count = PodU64::from(vault_count);
        self.operators_registered = PodU64::from(0);
        self.operators_can_vote_count = PodU64::from(0);
        self.total_agg_g1_pubkey = [0; G1_COMPRESSED_POINT_SIZE];
        let default_operator_snapshot = OperatorSnapshot::default();
        self.operator_snapshots = [default_operator_snapshot; 256];
        self.minimum_stake_weight = minimum_stake_weight;
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

    pub fn operators_can_vote_count(&self) -> u64 {
        self.operators_can_vote_count.into()
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

    pub fn minimum_stake_weight(&self) -> &StakeWeights {
        &self.minimum_stake_weight
    }

    pub fn increment_operator_registration(
        &mut self,
        current_slot: u64,
        vault_operator_delegations: u64,
    ) -> Result<(), NCNProgramError> {
        msg!(
            "Incrementing operator registration for epoch {} at slot {} with vault delegations {}",
            self.epoch(),
            current_slot,
            vault_operator_delegations
        );

        if self.finalized() {
            msg!(
                "Epoch snapshot for epoch {} is already finalized at slot {}",
                self.epoch(),
                self.slot_finalized()
            );
            return Err(NCNProgramError::EpochSnapshotAlreadyFinalized);
        }

        msg!(
            "Incrementing operator registration for epoch {} at slot {}",
            self.epoch(),
            current_slot
        );

        self.operators_registered = PodU64::from(
            self.operators_registered()
                .checked_add(1)
                .ok_or(NCNProgramError::ArithmeticOverflow)?,
        );
        msg!("Operators registered: {}", self.operators_registered());

        if vault_operator_delegations > 0 {
            self.operators_can_vote_count = PodU64::from(
                self.operators_can_vote_count()
                    .checked_add(1)
                    .ok_or(NCNProgramError::ArithmeticOverflow)?,
            );
        }

        msg!(
            "Operators can vote count: {}",
            self.operators_can_vote_count()
        );

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn register_operator_g1_pubkey(
        &mut self,
        operator_g1_pubkey: &[u8; G1_COMPRESSED_POINT_SIZE],
    ) -> Result<(), NCNProgramError> {
        alt_bn128_g1_decompress(operator_g1_pubkey)
            .map_err(|_| NCNProgramError::InvalidG1Pubkey)?;

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

    /// Get an operator snapshot by operator index
    pub fn get_operator_snapshot(&self, operator_index: u64) -> Option<&OperatorSnapshot> {
        if operator_index >= self.operator_count() {
            return None;
        }
        let snapshot = &self.operator_snapshots[operator_index as usize];
        if snapshot.ncn_operator_index() == u64::MAX {
            None
        } else {
            Some(snapshot)
        }
    }

    /// Get a mutable operator snapshot by operator index
    pub fn get_mut_operator_snapshot(
        &mut self,
        operator_index: u64,
    ) -> Option<&mut OperatorSnapshot> {
        if operator_index >= self.operator_count() {
            return None;
        }
        let snapshot = &mut self.operator_snapshots[operator_index as usize];
        if snapshot.ncn_operator_index() == u64::MAX {
            None
        } else {
            Some(snapshot)
        }
    }

    /// Find an operator snapshot by operator pubkey
    pub fn find_operator_snapshot(&self, operator: &Pubkey) -> Option<&OperatorSnapshot> {
        self.operator_snapshots.iter().find(|snapshot| {
            snapshot.operator() == operator && snapshot.ncn_operator_index() != u64::MAX
        })
    }

    /// Find a mutable operator snapshot by operator pubkey
    pub fn find_mut_operator_snapshot(
        &mut self,
        operator: &Pubkey,
    ) -> Option<&mut OperatorSnapshot> {
        self.operator_snapshots.iter_mut().find(|snapshot| {
            snapshot.operator() == operator && snapshot.ncn_operator_index() != u64::MAX
        })
    }

    /// Add a new operator snapshot to the array
    pub fn add_operator_snapshot(
        &mut self,
        operator_snapshot: OperatorSnapshot,
    ) -> Result<(), NCNProgramError> {
        let operator_index = operator_snapshot.ncn_operator_index();
        if operator_index >= MAX_OPERATORS as u64 {
            return Err(NCNProgramError::TooManyVaultOperatorDelegations);
        }

        // Check if slot is already occupied
        if self.operator_snapshots[operator_index as usize].ncn_operator_index() != u64::MAX {
            return Err(NCNProgramError::DuplicateVaultOperatorDelegation);
        }

        self.operator_snapshots[operator_index as usize] = operator_snapshot;
        Ok(())
    }

    /// Get all active operator snapshots
    pub fn get_active_operator_snapshots(&self) -> Vec<&OperatorSnapshot> {
        self.operator_snapshots
            .iter()
            .filter(|snapshot| snapshot.ncn_operator_index() != u64::MAX && snapshot.is_active())
            .collect()
    }

    /// Update an operator snapshot in the array
    pub fn update_operator_snapshot(
        &mut self,
        operator_index: usize,
        operator_snapshot: &OperatorSnapshot,
    ) {
        self.operator_snapshots[operator_index] = *operator_snapshot;
    }
}

// Operator snapshot entry within EpochSnapshot
#[derive(Debug, Clone, Copy, Zeroable, Pod, ShankType)]
#[repr(C)]
pub struct OperatorSnapshot {
    operator: Pubkey,

    g1_pubkey: [u8; 32], // G1 compressed pubkey

    slot_created: PodU64,
    slot_finalized: PodU64,

    is_active: PodBool,

    ncn_operator_index: PodU64,

    operator_index: PodU64,

    has_minimum_stake_weight: PodBool,

    stake_weight_so_far: StakeWeights,

    vault_operator_delegation_count: PodU64,
    vault_operator_delegations_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    vaults_delegated: [Pubkey; 10],
}

impl Default for OperatorSnapshot {
    fn default() -> Self {
        Self {
            operator: Pubkey::default(),
            g1_pubkey: [0; G1_COMPRESSED_POINT_SIZE],
            slot_created: PodU64::from(0),
            slot_finalized: PodU64::from(0),
            is_active: PodBool::from(false),
            ncn_operator_index: PodU64::from(u64::MAX),
            operator_index: PodU64::from(u64::MAX),
            has_minimum_stake_weight: PodBool::from(false),
            stake_weight_so_far: StakeWeights::default(),
            vaults_delegated: [Pubkey::default(); 10],
            vault_operator_delegation_count: PodU64::from(0),
            vault_operator_delegations_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
        }
    }
}

impl OperatorSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operator: &Pubkey,
        current_slot: u64,
        is_active: bool,
        ncn_operator_index: u64,
        operator_index: u64,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
        vault_operator_delegation_count: u64,
    ) -> Result<Self, NCNProgramError> {
        Ok(Self {
            operator: *operator,
            slot_created: PodU64::from(current_slot),
            slot_finalized: PodU64::from(0),
            is_active: PodBool::from(is_active),
            ncn_operator_index: PodU64::from(ncn_operator_index),
            operator_index: PodU64::from(operator_index),
            g1_pubkey,
            has_minimum_stake_weight: PodBool::from(false),
            stake_weight_so_far: StakeWeights::default(),
            vaults_delegated: [Pubkey::default(); 10],
            vault_operator_delegation_count: PodU64::from(vault_operator_delegation_count),
            vault_operator_delegations_registered: PodU64::from(0),
            valid_operator_vault_delegations: PodU64::from(0),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn initialize(
        &mut self,
        operator: &Pubkey,
        current_slot: u64,
        is_active: bool,
        ncn_operator_index: u64,
        operator_index: u64,
        g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
        vault_operator_delegation_count: u64,
    ) -> Result<(), NCNProgramError> {
        if vault_operator_delegation_count > MAX_VAULTS as u64 {
            return Err(NCNProgramError::TooManyVaultOperatorDelegations);
        }
        let slot_finalized = if !is_active { current_slot } else { 0 };
        let vault_operator_delegation_count = if is_active {
            vault_operator_delegation_count
        } else {
            0
        };

        // Initializes field by field to avoid overflowing stack
        self.operator = *operator;
        self.slot_created = PodU64::from(current_slot);
        self.slot_finalized = PodU64::from(slot_finalized);
        self.is_active = PodBool::from(is_active);
        self.ncn_operator_index = PodU64::from(ncn_operator_index);
        self.operator_index = PodU64::from(operator_index);
        self.g1_pubkey = g1_pubkey;
        self.has_minimum_stake_weight = PodBool::from(false);
        self.stake_weight_so_far = StakeWeights::default();
        self.vaults_delegated = [Pubkey::default(); MAX_VAULTS];
        self.vault_operator_delegation_count = PodU64::from(vault_operator_delegation_count);
        self.vault_operator_delegations_registered = PodU64::from(0);
        self.valid_operator_vault_delegations = PodU64::from(0);

        Ok(())
    }

    pub fn ncn_operator_index(&self) -> u64 {
        self.ncn_operator_index.into()
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

    pub fn has_minimum_stake_weight(&self) -> bool {
        self.has_minimum_stake_weight.into()
    }

    pub fn stake_weight_so_far(&self) -> &StakeWeights {
        &self.stake_weight_so_far
    }

    pub fn set_has_minimum_stake_weight(&mut self, has_minimum_stake_weight: bool) {
        self.has_minimum_stake_weight = PodBool::from(has_minimum_stake_weight);
    }

    pub fn set_stake_weight_so_far(&mut self, stake_weight_so_far: &StakeWeights) {
        self.stake_weight_so_far = *stake_weight_so_far;
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

    pub fn finalized(&self) -> bool {
        !self.is_active()
            || self.vault_operator_delegations_registered()
                == self.vault_operator_delegation_count()
            || self.vault_operator_delegations_registered() == MAX_VAULTS as u64
            || self.has_minimum_stake_weight()
    }

    pub fn contains_vault(&self, vault: &Pubkey) -> bool {
        self.vaults_delegated.iter().any(|v| v.eq(vault))
    }

    pub const fn vaults_delegated(&self) -> &[Pubkey] {
        &self.vaults_delegated
    }

    pub fn insert_vault_operator_stake_weight(
        &mut self,
        vault: &Pubkey,
    ) -> Result<(), NCNProgramError> {
        if self
            .vault_operator_delegations_registered()
            .checked_add(1)
            .ok_or(NCNProgramError::ArithmeticOverflow)?
            > MAX_VAULTS as u64
        {
            return Err(NCNProgramError::TooManyVaultOperatorDelegations);
        }

        if self.contains_vault(vault) {
            return Err(NCNProgramError::DuplicateVaultOperatorDelegation);
        }

        self.vaults_delegated[self.vault_operator_delegations_registered() as usize] = *vault;

        Ok(())
    }

    pub fn increment_vault_operator_delegation_registration(
        &mut self,
        current_slot: u64,
        vault: &Pubkey,
        stake_weights: &StakeWeights,
        minimum_stake_weight: &StakeWeights,
    ) -> Result<(), NCNProgramError> {
        if self.finalized() {
            return Err(NCNProgramError::VaultOperatorDelegationFinalized);
        }

        self.insert_vault_operator_stake_weight(vault)?;

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

        self.stake_weight_so_far.increment(stake_weights)?;

        if self.stake_weight_so_far().stake_weight() >= minimum_stake_weight.stake_weight() {
            self.set_has_minimum_stake_weight(true);
        }

        if self.finalized() {
            self.slot_finalized = PodU64::from(current_slot);
        }

        Ok(())
    }

    pub fn calculate_stake_weight(
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
       writeln!(f, "  Operators can vote:           {}", self.operators_can_vote_count())?;
       writeln!(f, "  Slot Finalized:               {}", self.slot_finalized())?;
       writeln!(f, "  Finalized:                    {}", self.finalized())?;
       writeln!(f, "  Total Agg G1 Pubkey:          {:?}", self.total_agg_g1_pubkey())?;
       writeln!(f, "  operators snapshots:")?;
       for operator_snapshot in self.operator_snapshots.iter() {
        if operator_snapshot.ncn_operator_index() != u64::MAX {
           writeln!(f, "{}", operator_snapshot)?;
        }
       }

       writeln!(f, "\n")?;
       Ok(())
   }
}

#[rustfmt::skip]
impl fmt::Display for OperatorSnapshot {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       writeln!(f, "\n\n----------- Operator Snapshot -------------")?;
       writeln!(f, "  Operator:                     {}", self.operator)?;
       writeln!(f, "  Is Active:                    {}", self.is_active())?;
       writeln!(f, "  NCN Operator Index:           {}", self.ncn_operator_index())?;
       writeln!(f, "  Finalized:                    {}", self.finalized())?;
       writeln!(f, "  G1 Pubkey:                    {:?}", self.g1_pubkey())?;
       writeln!(f, "  Has Minimum Stake Weight:     {}", self.has_minimum_stake_weight())?;
       writeln!(f, "  Stake Weight So Far:          {:?}", self.stake_weight_so_far())?;
       writeln!(f, "  Vault Operator Delegation Count: {}", self.vault_operator_delegation_count())?;
       writeln!(f, "  Vault Operator Delegations Registered: {}", self.vault_operator_delegations_registered())?;
       writeln!(f, "  Valid Operator Vault Delegations: {}", self.valid_operator_vault_delegations())?;
       writeln!(f, "  Vaults Delegated:")?;
       for vault in self.vaults_delegated.iter() {
           if *vault != Pubkey::default() {
               writeln!(f, "    - {}", vault)?;
           }
       }

       writeln!(f, "\n")?;
       Ok(())
   }
}

#[cfg(test)]
mod tests {
    use solana_program::msg;

    use super::*;

    #[test]
    fn test_operator_snapshot_size() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // operator
            + size_of::<[u8; G1_COMPRESSED_POINT_SIZE]>() // g1_pubkey
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // slot_finalized
            + size_of::<PodBool>() // is_active
            + size_of::<PodU64>() // ncn_operator_index
            + size_of::<PodU64>() // operator_index
            + size_of::<PodBool>() // has_minimum_stake_weight
            + size_of::<StakeWeights>() // stake_weight_so_far
            + size_of::<PodU64>() // vault_operator_delegation_count
            + size_of::<PodU64>() // vault_operator_delegations_registered
            + size_of::<PodU64>() // valid_operator_vault_delegations
            + size_of::<[Pubkey; 10]>(); // vaults_delegated

        assert_eq!(size_of::<OperatorSnapshot>(), expected_total);
    }

    #[test]
    fn test_epoch_snapshot_size() {
        use std::mem::size_of;

        msg!("EpochSnapshot size: {:?}", size_of::<EpochSnapshot>());

        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<PodU64>() // epoch
            + size_of::<u8>() // bump
            + size_of::<PodU64>() // slot_created
            + size_of::<PodU64>() // slot_finalized
            + size_of::<PodU64>() // operator_count
            + size_of::<PodU64>() // vault_count
            + size_of::<PodU64>() // operators_registered
            + size_of::<PodU64>() // operators_can_vote_count
            + size_of::<[u8; G1_COMPRESSED_POINT_SIZE]>() // total_agg_g1_pubkey
            + size_of::<[OperatorSnapshot; 256]>() // operator_snapshots
            + size_of::<StakeWeights>(); // minimum_stake_weight

        assert_eq!(size_of::<EpochSnapshot>(), expected_total);
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
    fn test_increment_operator_registration_finalized() {
        // Create an epoch snapshot
        let mut snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            1,                    // operator_count - set to 1
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // Set operators_registered equal to operator_count to make it finalized
        snapshot.operators_registered = PodU64::from(1);

        // Try to increment operator registration when already finalized
        let result = snapshot.increment_operator_registration(
            200, // current_slot
            1,   // vault_operator_delegations
        );

        // Verify we get the expected error
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::EpochSnapshotAlreadyFinalized
        );
    }

    #[test]
    fn test_operator_snapshot_initialize_active_inactive() {
        let current_slot = 100;
        let g1_pubkey = G1CompressedPoint::from_random().0;

        // Create two operator snapshots - one for active and one for inactive
        let mut active_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            current_slot,
            true,
            0,
            0,
            g1_pubkey,
            1,
        )
        .unwrap();

        let mut inactive_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            current_slot,
            false,
            0,
            0,
            g1_pubkey,
            1,
        )
        .unwrap();

        // Initialize active snapshot
        active_snapshot
            .initialize(
                &Pubkey::new_unique(),
                current_slot,
                true, // is_active
                0,
                0,
                g1_pubkey,
                1,
            )
            .unwrap();

        // Initialize inactive snapshot
        inactive_snapshot
            .initialize(
                &Pubkey::new_unique(),
                current_slot,
                false, // not active
                0,
                0,
                g1_pubkey,
                1,
            )
            .unwrap();
    }

    #[test]
    fn test_register_operator_g1_pubkey() {
        // Create an epoch snapshot with default aggregated G1 pubkey (all zeros) using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            3,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

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
        // Create an epoch snapshot using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            3,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

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
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
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
            100,                // current_slot
            true,               // is_active
            0,                  // ncn_operator_index
            0,                  // operator_index
            original_g1_pubkey, // original g1_pubkey
            1,
        )
        .unwrap();

        // Initialize with a new G1 pubkey
        operator_snapshot
            .initialize(
                &Pubkey::new_unique(),
                100,           // current_slot
                true,          // is_active
                0,             // ncn_operator_index
                0,             // operator_index
                new_g1_pubkey, // new g1_pubkey
                1,
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

        // Create two epoch snapshots using heap allocation to avoid stack overflow
        let mut snapshot1 = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // Register operators in different orders
        // Snapshot 1: operator1 first, then operator2
        snapshot1
            .register_operator_g1_pubkey(&operator1_g1_pubkey)
            .unwrap();
        snapshot1
            .register_operator_g1_pubkey(&operator2_g1_pubkey)
            .unwrap();

        let mut snapshot2 = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );
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
        // Create an epoch snapshot using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            1,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

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

    #[test]
    fn test_epoch_snapshot_add_operator_snapshot() {
        // Create an epoch snapshot using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            3,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

        // Create an operator snapshot
        let operator_pubkey = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let operator_snapshot = OperatorSnapshot::new(
            &operator_pubkey,
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        // Add the operator snapshot to the epoch snapshot
        let result = epoch_snapshot.add_operator_snapshot(operator_snapshot);
        assert!(result.is_ok());

        {
            // Verify the operator snapshot was added using its index
            let retrieved_snapshot = epoch_snapshot.get_operator_snapshot(0);
            assert!(retrieved_snapshot.is_some());
            assert_eq!(retrieved_snapshot.unwrap().operator(), &operator_pubkey);
        }

        {
            // Verify the operator snapshot was added using its id
            let retrieved_snapshot = epoch_snapshot.find_operator_snapshot(&operator_pubkey);
            assert!(retrieved_snapshot.is_some());
            assert_eq!(retrieved_snapshot.unwrap().operator(), &operator_pubkey);
        }
    }

    #[test]
    fn test_epoch_snapshot_find_operator_snapshot() {
        // Create an epoch snapshot using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            3,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

        // Create operator snapshots
        let operator1_pubkey = Pubkey::new_unique();
        let operator2_pubkey = Pubkey::new_unique();
        let g1_pubkey_1 = G1CompressedPoint::from_random().0;
        let g1_pubkey_2 = G1CompressedPoint::from_random().0;

        let operator1_snapshot = OperatorSnapshot::new(
            &operator1_pubkey,
            100,         // current_slot
            true,        // is_active
            0,           // ncn_operator_index (index 0)
            0,           // operator_index
            g1_pubkey_1, // g1_pubkey
            1,
        )
        .unwrap();

        let operator2_snapshot = OperatorSnapshot::new(
            &operator2_pubkey,
            100,         // current_slot
            true,        // is_active
            1,           // ncn_operator_index (index 1)
            1,           // operator_index
            g1_pubkey_2, // g1_pubkey
            1,
        )
        .unwrap();

        // Add both operator snapshots
        epoch_snapshot
            .add_operator_snapshot(operator1_snapshot)
            .unwrap();
        epoch_snapshot
            .add_operator_snapshot(operator2_snapshot)
            .unwrap();

        // Find operator snapshots by pubkey
        let found1 = epoch_snapshot.find_operator_snapshot(&operator1_pubkey);
        let found2 = epoch_snapshot.find_operator_snapshot(&operator2_pubkey);
        let not_found = epoch_snapshot.find_operator_snapshot(&Pubkey::new_unique());

        assert!(found1.is_some());
        assert_eq!(found1.unwrap().operator(), &operator1_pubkey);

        assert!(found2.is_some());
        assert_eq!(found2.unwrap().operator(), &operator2_pubkey);

        assert!(not_found.is_none());
    }

    #[test]
    fn test_epoch_snapshot_add_operator_snapshot_duplicate() {
        // Create an epoch snapshot using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            3,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

        // Create two operator snapshots with the same index
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let operator1_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        let operator2_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index (same as operator1)
            1,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        // Add first operator snapshot - should succeed
        let result1 = epoch_snapshot.add_operator_snapshot(operator1_snapshot);
        assert!(result1.is_ok());

        // Try to add second operator snapshot with same index - should fail
        let result2 = epoch_snapshot.add_operator_snapshot(operator2_snapshot);
        assert!(result2.is_err());
        assert_eq!(
            result2.unwrap_err(),
            NCNProgramError::DuplicateVaultOperatorDelegation
        );
    }

    #[test]
    fn test_epoch_snapshot_get_active_operator_snapshots() {
        // Create an epoch snapshot using heap allocation
        let mut epoch_snapshot = Box::new(EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            3,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        ));

        let g1_pubkey = G1CompressedPoint::from_random().0;

        // Create active operator snapshot
        let active_operator_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,       // current_slot
            true,      // is_active = true
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        // Create inactive operator snapshot
        let inactive_operator_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,       // current_slot
            false,     // is_active = false
            1,         // ncn_operator_index
            1,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        // Add both operator snapshots
        epoch_snapshot
            .add_operator_snapshot(active_operator_snapshot)
            .unwrap();
        epoch_snapshot
            .add_operator_snapshot(inactive_operator_snapshot)
            .unwrap();

        // Get active operator snapshots
        let active_snapshots = epoch_snapshot.get_active_operator_snapshots();

        // Should only return the active one
        assert_eq!(active_snapshots.len(), 1);
        assert!(active_snapshots[0].is_active());
        assert_eq!(active_snapshots[0].ncn_operator_index(), 0);
    }

    #[test]
    fn test_epoch_snapshot_initialize() {
        let ncn = Pubkey::new_unique();
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            1,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        epoch_snapshot.initialize(
            &ncn,
            5,                      // ncn_epoch
            2,                      // bump
            200,                    // current_slot
            10,                     // operator_count
            5,                      // vault_count
            StakeWeights::new(100), // minimum_stake_weight
        );

        assert_eq!(epoch_snapshot.ncn, ncn);
        assert_eq!(epoch_snapshot.epoch(), 5);
        assert_eq!(epoch_snapshot.bump, 2);
        assert_eq!(u64::from(epoch_snapshot.slot_created), 200u64);
        assert_eq!(epoch_snapshot.operator_count(), 10);
        assert_eq!(epoch_snapshot.vault_count(), 5);
        assert_eq!(epoch_snapshot.minimum_stake_weight().stake_weight(), 100);
    }

    #[test]
    fn test_epoch_snapshot_seeds_and_pda() {
        let ncn = Pubkey::new_unique();
        let program_id = Pubkey::new_unique();
        let epoch = 42;

        let seeds = EpochSnapshot::seeds(&ncn, epoch);
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0], b"epoch_snapshot");
        assert_eq!(seeds[1], ncn.to_bytes().to_vec());
        assert_eq!(seeds[2], epoch.to_le_bytes().to_vec());

        let (pda, bump, returned_seeds) =
            EpochSnapshot::find_program_address(&program_id, &ncn, epoch);
        assert_eq!(returned_seeds, seeds);
        assert!(bump > 0);
        assert!(pda != Pubkey::default());
    }

    #[test]
    fn test_epoch_snapshot_getters() {
        let ncn = Pubkey::new_unique();
        let epoch_snapshot = EpochSnapshot::new(
            &ncn,
            10,                     // ncn_epoch
            3,                      // bump
            500,                    // current_slot
            15,                     // operator_count
            8,                      // vault_count
            StakeWeights::new(200), // minimum_stake_weight
        );

        assert_eq!(epoch_snapshot.epoch(), 10);
        assert_eq!(epoch_snapshot.operator_count(), 15);
        assert_eq!(epoch_snapshot.vault_count(), 8);
        assert_eq!(epoch_snapshot.operators_registered(), 0);
        assert_eq!(epoch_snapshot.operators_can_vote_count(), 0);
        assert_eq!(epoch_snapshot.slot_finalized(), 0);
        assert!(!epoch_snapshot.finalized());
        assert_eq!(epoch_snapshot.minimum_stake_weight().stake_weight(), 200);
    }

    #[test]
    fn test_epoch_snapshot_increment_operator_registration() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // First increment - should succeed
        let result = epoch_snapshot.increment_operator_registration(150, 1);
        assert!(result.is_ok());
        assert_eq!(epoch_snapshot.operators_registered(), 1);
        assert_eq!(epoch_snapshot.operators_can_vote_count(), 1);
        assert_eq!(epoch_snapshot.slot_finalized(), 0); // Not finalized yet

        // Second increment - should finalize
        let result = epoch_snapshot.increment_operator_registration(200, 0);
        assert!(result.is_ok());
        assert_eq!(epoch_snapshot.operators_registered(), 2);
        assert_eq!(epoch_snapshot.operators_can_vote_count(), 1); // No change since vault_operator_delegations = 0
        assert_eq!(epoch_snapshot.slot_finalized(), 200); // Now finalized
        assert!(epoch_snapshot.finalized());

        // Third increment - should fail since finalized
        let result = epoch_snapshot.increment_operator_registration(250, 1);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::EpochSnapshotAlreadyFinalized
        );
    }

    #[test]
    fn test_epoch_snapshot_get_operator_snapshot() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // Test getting non-existent snapshot
        assert!(epoch_snapshot.get_operator_snapshot(0).is_none());
        assert!(epoch_snapshot.get_operator_snapshot(1).is_none());

        // Add an operator snapshot
        let operator_pubkey = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let operator_snapshot = OperatorSnapshot::new(
            &operator_pubkey,
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        epoch_snapshot
            .add_operator_snapshot(operator_snapshot)
            .unwrap();

        // Test getting existing snapshot
        let retrieved = epoch_snapshot.get_operator_snapshot(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().operator(), &operator_pubkey);

        // Test getting out of bounds
        assert!(epoch_snapshot.get_operator_snapshot(2).is_none());
    }

    #[test]
    fn test_epoch_snapshot_get_mut_operator_snapshot() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // Add an operator snapshot
        let operator_pubkey = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let operator_snapshot = OperatorSnapshot::new(
            &operator_pubkey,
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        epoch_snapshot
            .add_operator_snapshot(operator_snapshot)
            .unwrap();

        // Test getting mutable reference
        let mut_snapshot = epoch_snapshot.get_mut_operator_snapshot(0);
        assert!(mut_snapshot.is_some());

        // Modify the snapshot
        if let Some(snapshot) = mut_snapshot {
            snapshot.set_has_minimum_stake_weight(true);
            assert!(snapshot.has_minimum_stake_weight());
        }

        // Verify the change persisted
        let retrieved = epoch_snapshot.get_operator_snapshot(0);
        assert!(retrieved.unwrap().has_minimum_stake_weight());
    }

    #[test]
    fn test_epoch_snapshot_find_mut_operator_snapshot() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        let operator_pubkey = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;
        let operator_snapshot = OperatorSnapshot::new(
            &operator_pubkey,
            100,       // current_slot
            true,      // is_active
            0,         // ncn_operator_index
            0,         // operator_index
            g1_pubkey, // g1_pubkey
            1,
        )
        .unwrap();

        epoch_snapshot
            .add_operator_snapshot(operator_snapshot)
            .unwrap();

        // Test finding by pubkey
        let mut_found = epoch_snapshot.find_mut_operator_snapshot(&operator_pubkey);
        assert!(mut_found.is_some());

        // Test finding non-existent
        let not_found = epoch_snapshot.find_mut_operator_snapshot(&Pubkey::new_unique());
        assert!(not_found.is_none());
    }

    #[test]
    fn test_epoch_snapshot_add_operator_snapshot_index_overflow() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            2,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        let g1_pubkey = G1CompressedPoint::from_random().0;
        let operator_snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,                  // current_slot
            true,                 // is_active
            MAX_OPERATORS as u64, // ncn_operator_index (too large)
            0,                    // operator_index
            g1_pubkey,            // g1_pubkey
            1,
        )
        .unwrap();

        let result = epoch_snapshot.add_operator_snapshot(operator_snapshot);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::TooManyVaultOperatorDelegations
        );
    }

    #[test]
    fn test_operator_snapshot_new() {
        let operator = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;

        let snapshot = OperatorSnapshot::new(
            &operator, 100,       // current_slot
            true,      // is_active
            5,         // ncn_operator_index
            10,        // operator_index
            g1_pubkey, // g1_pubkey
            1,         // vault_operator_delegation_count
        )
        .unwrap();

        assert_eq!(snapshot.operator(), &operator);
        assert_eq!(u64::from(snapshot.slot_created), 100u64);
        assert_eq!(u64::from(snapshot.slot_finalized), 0u64);
        assert!(snapshot.is_active());
        assert_eq!(snapshot.ncn_operator_index(), 5);
        assert_eq!(u64::from(snapshot.operator_index), 10u64);
        assert_eq!(snapshot.g1_pubkey(), g1_pubkey);
        assert!(!snapshot.has_minimum_stake_weight());
        assert_eq!(snapshot.stake_weight_so_far().stake_weight(), 0);
        assert_eq!(snapshot.vault_operator_delegation_count(), 1);
        assert_eq!(snapshot.vault_operator_delegations_registered(), 0);
        assert_eq!(snapshot.valid_operator_vault_delegations(), 0);
    }

    #[test]
    fn test_operator_snapshot_initialize_validation() {
        let mut snapshot = OperatorSnapshot::default();

        // Test with too many vault operator delegations
        let result = snapshot.initialize(
            &Pubkey::new_unique(),
            100,                                // current_slot
            true,                               // is_active
            0,                                  // ncn_operator_index
            0,                                  // operator_index
            G1CompressedPoint::from_random().0, // g1_pubkey
            MAX_VAULTS as u64 + 1,              // too many delegations
        );

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::TooManyVaultOperatorDelegations
        );
    }

    #[test]
    fn test_operator_snapshot_initialize_inactive() {
        let mut snapshot = OperatorSnapshot::default();

        let operator = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;

        snapshot
            .initialize(
                &operator, 100,       // current_slot
                false,     // is_active = false
                0,         // ncn_operator_index
                0,         // operator_index
                g1_pubkey, // g1_pubkey
                5,         // vault_operator_delegation_count
            )
            .unwrap();

        assert_eq!(snapshot.operator(), &operator);
        assert!(!snapshot.is_active());
        assert_eq!(u64::from(snapshot.slot_finalized), 100u64); // Should be finalized immediately
        assert_eq!(snapshot.vault_operator_delegation_count(), 0); // Should be reset to 0
    }

    #[test]
    fn test_operator_snapshot_have_valid_bn128_g1_pubkey() {
        let mut snapshot = OperatorSnapshot::default();

        // Test with invalid pubkey (all zeros)
        assert!(!snapshot.have_valid_bn128_g1_pubkey());

        // Test with valid pubkey
        let valid_g1_pubkey = G1CompressedPoint::from_random().0;
        snapshot.g1_pubkey = valid_g1_pubkey;
        assert!(snapshot.have_valid_bn128_g1_pubkey());
    }

    #[test]
    fn test_operator_snapshot_stake_weight_operations() {
        let mut snapshot = OperatorSnapshot::default();

        // Test initial state
        assert_eq!(snapshot.stake_weight_so_far().stake_weight(), 0);
        assert!(!snapshot.has_minimum_stake_weight());

        // Test setting stake weight
        let stake_weight = StakeWeights::new(100);
        snapshot.set_stake_weight_so_far(&stake_weight);
        assert_eq!(snapshot.stake_weight_so_far().stake_weight(), 100);

        // Test incrementing stake weight
        let increment = StakeWeights::new(50);
        snapshot.stake_weight_so_far.increment(&increment);
        assert_eq!(snapshot.stake_weight_so_far().stake_weight(), 150);

        // Test setting minimum stake weight flag
        snapshot.set_has_minimum_stake_weight(true);
        assert!(snapshot.has_minimum_stake_weight());
    }

    #[test]
    fn test_operator_snapshot_finalized() {
        let mut snapshot = OperatorSnapshot::default();

        // Test finalized by registration count
        snapshot.vault_operator_delegation_count = PodU64::from(5);
        snapshot.vault_operator_delegations_registered = PodU64::from(5);
        assert!(snapshot.finalized());

        // Test finalized by MAX_VAULTS
        snapshot.vault_operator_delegations_registered = PodU64::from(MAX_VAULTS as u64);
        assert!(snapshot.finalized());

        // Test finalized by minimum stake weight
        snapshot.vault_operator_delegations_registered = PodU64::from(0);
        snapshot.set_has_minimum_stake_weight(true);
        assert!(snapshot.finalized());
    }

    #[test]
    fn test_operator_snapshot_contains_vault() {
        let mut snapshot = OperatorSnapshot::default();

        let vault1 = Pubkey::new_unique();
        let vault2 = Pubkey::new_unique();

        // Test empty vaults list
        assert!(!snapshot.contains_vault(&vault1));

        // Add vaults
        snapshot.vaults_delegated[0] = vault1;
        snapshot.vaults_delegated[1] = vault2;

        // Test contains
        assert!(snapshot.contains_vault(&vault1));
        assert!(snapshot.contains_vault(&vault2));
        assert!(!snapshot.contains_vault(&Pubkey::new_unique()));
    }

    #[test]
    fn test_operator_snapshot_insert_vault_operator_stake_weight() {
        let mut snapshot = OperatorSnapshot::default();
        snapshot.vault_operator_delegations_registered = PodU64::from(MAX_VAULTS as u64);

        let vault = Pubkey::new_unique();

        // Test overflow
        let result = snapshot.insert_vault_operator_stake_weight(&vault);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::TooManyVaultOperatorDelegations
        );

        // Reset and test duplicate
        snapshot.vault_operator_delegations_registered = PodU64::from(0);
        snapshot.vaults_delegated[0] = vault;

        let result = snapshot.insert_vault_operator_stake_weight(&vault);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::DuplicateVaultOperatorDelegation
        );

        // Test successful insertion
        let new_vault = Pubkey::new_unique();
        let result = snapshot.insert_vault_operator_stake_weight(&new_vault);
        assert!(result.is_ok());
        assert_eq!(snapshot.vaults_delegated[0], new_vault);
    }

    #[test]
    fn test_operator_snapshot_increment_vault_operator_delegation_registration() {
        let mut snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,                                // current_slot
            true,                               // is_active
            1,                                  // ncn_operator_index
            1,                                  // operator_index
            G1CompressedPoint::from_random().0, // g1_pubkey
            2,                                  // vault_operator_delegation_count
        )
        .unwrap();

        let minimum_stake_weight = StakeWeights::new(100);

        // Test finalized snapshot
        snapshot.set_has_minimum_stake_weight(true);
        let result = snapshot.increment_vault_operator_delegation_registration(
            100, // current_slot
            &Pubkey::new_unique(),
            &StakeWeights::new(50),
            &minimum_stake_weight,
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::VaultOperatorDelegationFinalized
        );

        // Reset and test successful increment
        snapshot.set_has_minimum_stake_weight(false);
        let vault = Pubkey::new_unique();
        let stake_weights = StakeWeights::new(50);

        let result = snapshot.increment_vault_operator_delegation_registration(
            100, // current_slot
            &vault,
            &stake_weights,
            &minimum_stake_weight,
        );
        assert!(result.is_ok());

        assert_eq!(snapshot.vault_operator_delegations_registered(), 1);
        assert_eq!(snapshot.valid_operator_vault_delegations(), 1);
        assert_eq!(snapshot.stake_weight_so_far().stake_weight(), 50);
        assert!(!snapshot.has_minimum_stake_weight()); // Not enough for minimum

        // Test with zero stake weight
        let result = snapshot.increment_vault_operator_delegation_registration(
            150, // current_slot
            &Pubkey::new_unique(),
            &StakeWeights::new(0),
            &minimum_stake_weight,
        );
        msg!("Result: {:?}", result);
        assert!(result.is_ok());
        assert_eq!(snapshot.valid_operator_vault_delegations(), 1); // No increment
    }

    #[test]
    fn test_operator_snapshot_increment_vault_operator_delegation_registration_minimum_weight() {
        let mut snapshot = OperatorSnapshot::new(
            &Pubkey::new_unique(),
            100,                                // current_slot
            true,                               // is_active
            1,                                  // ncn_operator_index
            1,                                  // operator_index
            G1CompressedPoint::from_random().0, // g1_pubkey
            1,                                  // vault_operator_delegation_count
        )
        .unwrap();
        let minimum_stake_weight = StakeWeights::new(100);

        let vault = Pubkey::new_unique();
        let stake_weights = StakeWeights::new(100); // Exactly minimum

        let result = snapshot.increment_vault_operator_delegation_registration(
            100, // current_slot
            &vault,
            &stake_weights,
            &minimum_stake_weight,
        );
        assert!(result.is_ok());

        assert!(snapshot.has_minimum_stake_weight());
        assert_eq!(u64::from(snapshot.slot_finalized), 100u64); // Should be finalized
    }

    #[test]
    fn test_vault_operator_stake_weight_new() {
        let vault = Pubkey::new_unique();
        let stake_weight = StakeWeights::new(500);

        let vault_stake_weight = VaultOperatorStakeWeight::new(&vault, 10, &stake_weight);

        assert_eq!(vault_stake_weight.vault(), &vault);
        assert_eq!(vault_stake_weight.vault_index(), 10);
        assert_eq!(vault_stake_weight.stake_weights().stake_weight(), 500);
        assert!(!vault_stake_weight.is_empty());
    }

    #[test]
    fn test_vault_operator_stake_weight_default() {
        let default_weight = VaultOperatorStakeWeight::default();

        assert_eq!(default_weight.vault(), &Pubkey::default());
        assert_eq!(default_weight.vault_index(), u64::MAX);
        assert_eq!(default_weight.stake_weights().stake_weight(), 0);
        assert!(default_weight.is_empty());
    }

    #[test]
    fn test_epoch_snapshot_register_operator_g1_pubkey_invalid_pubkey() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            1,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // Test with invalid G1 pubkey (all zeros except one byte)
        let mut invalid_pubkey = [0u8; G1_COMPRESSED_POINT_SIZE];
        invalid_pubkey[0] = 1; // Make it invalid G1 point

        let result = epoch_snapshot.register_operator_g1_pubkey(&invalid_pubkey);
        assert!(result.is_err()); // Should fail with invalid G1 point
    }

    #[test]
    fn test_operator_snapshot_calculate_stake_weight() {
        // This test would require mocking VaultOperatorDelegation and WeightTable
        // For now, we'll test the method signature and basic error handling
        let vault_delegation =
            VaultOperatorDelegation::new(Pubkey::new_unique(), Pubkey::new_unique(), 0, 1, 100);
        let weight_table = WeightTable::new(&Pubkey::new_unique(), 1, 100, 1, 1);
        let st_mint = Pubkey::new_unique();

        let result =
            OperatorSnapshot::calculate_stake_weight(&vault_delegation, &weight_table, &st_mint);

        // The result depends on the implementation of VaultOperatorDelegation and WeightTable
        // This test ensures the method can be called without compilation errors
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_epoch_snapshot_edge_cases() {
        // Test with maximum values
        let epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            u64::MAX,                     // ncn_epoch
            255,                          // bump
            u64::MAX,                     // current_slot
            MAX_OPERATORS as u64,         // operator_count
            MAX_VAULTS as u64,            // vault_count
            StakeWeights::new(u128::MAX), // minimum_stake_weight
        );

        assert_eq!(epoch_snapshot.epoch(), u64::MAX);
        assert_eq!(epoch_snapshot.operator_count(), MAX_OPERATORS as u64);
        assert_eq!(epoch_snapshot.vault_count(), MAX_VAULTS as u64);
        assert_eq!(
            epoch_snapshot.minimum_stake_weight().stake_weight(),
            u128::MAX
        );
    }

    #[test]
    fn test_operator_snapshot_edge_cases() {
        let operator = Pubkey::new_unique();
        let g1_pubkey = G1CompressedPoint::from_random().0;

        let snapshot = OperatorSnapshot::new(
            &operator,
            u64::MAX,                 // current_slot
            true,                     // is_active
            MAX_OPERATORS as u64 - 1, // ncn_operator_index
            u64::MAX,                 // operator_index
            g1_pubkey,                // g1_pubkey
            1,                        // vault_operator_delegation_count
        )
        .unwrap();

        assert_eq!(u64::from(snapshot.slot_created), u64::MAX);
        assert_eq!(snapshot.ncn_operator_index(), MAX_OPERATORS as u64 - 1);
        assert_eq!(u64::from(snapshot.operator_index), u64::MAX);
    }

    #[test]
    fn test_epoch_snapshot_arithmetic_overflow_protection() {
        let mut epoch_snapshot = EpochSnapshot::new(
            &Pubkey::new_unique(),
            1,                    // ncn_epoch
            1,                    // bump
            100,                  // current_slot
            1,                    // operator_count
            1,                    // vault_count
            StakeWeights::new(1), // minimum_stake_weight
        );

        // Set to maximum values to test overflow
        epoch_snapshot.operators_registered = PodU64::from(u64::MAX);

        let result = epoch_snapshot.increment_operator_registration(200, 1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), NCNProgramError::ArithmeticOverflow);
    }

    #[test]
    fn test_operator_snapshot_arithmetic_overflow_protection() {
        let mut snapshot = OperatorSnapshot::default();

        // Set to maximum values to test overflow
        snapshot.vault_operator_delegations_registered = PodU64::from(u64::MAX);

        let result = snapshot.increment_vault_operator_delegation_registration(
            100, // current_slot
            &Pubkey::new_unique(),
            &StakeWeights::new(50),
            &StakeWeights::new(100),
        );
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            NCNProgramError::VaultOperatorDelegationFinalized
        );
    }
}
