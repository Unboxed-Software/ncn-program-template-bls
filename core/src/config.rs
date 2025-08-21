use core::fmt;
use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{
    discriminators::Discriminators, fees::FeeConfig, loaders::check_load,
    stake_weight::StakeWeights,
};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum ConfigAdminRole {
    TieBreakerAdmin,
}

#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct Config {
    /// The Restaking program's NCN admin is the signer to create and update this account
    pub ncn: Pubkey,
    /// The admin to update the tie breaker - who can decide the meta merkle root when consensus is reached
    pub tie_breaker_admin: Pubkey,
    /// Number of slots after consensus reached where voting is still valid
    pub valid_slots_after_consensus: PodU64,
    /// Number of epochs before voting is considered stalled
    pub epochs_before_stall: PodU64,
    /// Number of epochs after consensus reached where voting is still valid
    pub epochs_after_consensus_before_close: PodU64,
    /// Only epochs after this epoch are valid for voting
    pub starting_valid_epoch: PodU64,
    /// The fee config
    pub fee_config: FeeConfig,
    /// Bump seed for the PDA
    pub bump: u8,
    /// Minimum stake weight required to vote
    pub minimum_stake: StakeWeights,
}

impl Discriminator for Config {
    const DISCRIMINATOR: u8 = Discriminators::Config as u8;
}

impl Config {
    const CONFIG_SEED: &'static [u8] = b"config";
    pub const SIZE: usize = 8 + size_of::<Self>();

    /// Creates a new Config instance with the specified parameters
    ///
    /// # Arguments
    /// * `ncn` - Pubkey of the NCN admin authority
    /// * `tie_breaker_admin` - Pubkey of the tie breaker admin
    /// * `starting_valid_epoch` - Epoch from which voting becomes valid
    /// * `valid_slots_after_consensus` - Number of slots after consensus where voting is still valid
    /// * `epochs_before_stall` - Number of epochs before system is considered stalled
    /// * `epochs_after_consensus_before_close` - Number of epochs after consensus before accounts can be closed
    /// * `bump` - Bump seed for PDA derivation
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        ncn: &Pubkey,
        tie_breaker_admin: &Pubkey,
        starting_valid_epoch: u64,
        valid_slots_after_consensus: u64,
        epochs_before_stall: u64,
        epochs_after_consensus_before_close: u64,
        fee_config: &FeeConfig,
        minimum_stake: &StakeWeights,
        bump: u8,
    ) -> Self {
        Self {
            ncn: *ncn,
            tie_breaker_admin: *tie_breaker_admin,
            starting_valid_epoch: PodU64::from(starting_valid_epoch),
            valid_slots_after_consensus: PodU64::from(valid_slots_after_consensus),
            epochs_before_stall: PodU64::from(epochs_before_stall),
            epochs_after_consensus_before_close: PodU64::from(epochs_after_consensus_before_close),
            fee_config: *fee_config,
            bump,
            minimum_stake: *minimum_stake,
        }
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        vec![Self::CONFIG_SEED.to_vec(), ncn.to_bytes().to_vec()]
    }

    pub fn find_program_address(program_id: &Pubkey, ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn);
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    /// Validates and loads the Config account - checks if it's the correct PDA and has the right discriminator
    ///
    /// # Arguments
    /// * `program_id` - The program ID
    /// * `ncn` - The NCN pubkey
    /// * `account` - The account to load
    /// * `expect_writable` - Whether the account should be writable
    ///
    /// # Returns
    /// * `Result<(), ProgramError>` - Ok if valid, Error otherwise
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

    pub fn starting_valid_epoch(&self) -> u64 {
        self.starting_valid_epoch.into()
    }

    pub fn valid_slots_after_consensus(&self) -> u64 {
        self.valid_slots_after_consensus.into()
    }

    pub fn epochs_before_stall(&self) -> u64 {
        self.epochs_before_stall.into()
    }

    pub fn epochs_after_consensus_before_close(&self) -> u64 {
        self.epochs_after_consensus_before_close.into()
    }

    pub fn minimum_stake(&self) -> &StakeWeights {
        &self.minimum_stake
    }
}

#[rustfmt::skip]
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\n\n----------- Config -------------")?;
        writeln!(f, "  NCN:                          {}", self.ncn)?;
        writeln!(f, "  Tie Breaker:                  {}", self.tie_breaker_admin)?;
        writeln!(f, "  Valid Slots After Consensus:  {}", self.valid_slots_after_consensus())?;
        writeln!(f, "  Epochs Before Stall:          {}", self.epochs_before_stall())?;
        writeln!(f, "  Starting Valid Epochs:        {}", self.starting_valid_epoch())?;
        writeln!(f, "  Close Epoch:                  {}", self.epochs_after_consensus_before_close())?;
        writeln!(f, "  Minimum Stake Weight:         {:?}", self.minimum_stake())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_len() {
        use std::mem::size_of;

        let expected_total = size_of::<Pubkey>() // ncn
            + size_of::<Pubkey>() // tie_breaker_admin 
            + size_of::<PodU64>() // valid_slots_after_consensus
            + size_of::<PodU64>() // epochs_before_stall
            + size_of::<PodU64>() // epochs_after_consensus_before_close
            + size_of::<PodU64>() // starting_valid_epoch
            + size_of::<FeeConfig>() // fee_config
            + 1 // bump
            + size_of::<StakeWeights>(); // minimum_stake

        assert_eq!(size_of::<Config>(), expected_total);
        assert_eq!(size_of::<Config>() + 8, Config::SIZE);
    }
}
