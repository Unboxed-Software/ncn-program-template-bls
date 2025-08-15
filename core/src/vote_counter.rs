use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::ShankAccount;
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{discriminators::Discriminators, loaders::check_load};

/// Vote counter PDA that tracks the number of successful votes for an NCN
/// This counter is incremented each time a vote instruction passes successfully
/// and can be used to prevent duplicate signatures by using the counter as the message
#[derive(Debug, Clone, Copy, Zeroable, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct VoteCounter {
    /// The NCN this counter belongs to
    pub ncn: Pubkey,
    /// Current count of successful votes
    pub count: PodU64,
    /// Bump seed for the PDA
    pub bump: u8,
    /// Reserved bytes for future use
    pub reserved: [u8; 7],
}

impl Discriminator for VoteCounter {
    const DISCRIMINATOR: u8 = Discriminators::VoteCounter as u8;
}

impl VoteCounter {
    pub const LEN: usize = 32 + 8 + 1 + 7; // ncn + count + bump + reserved
    pub const SIZE: usize = 8 + Self::LEN; // discriminator + data

    /// Create a new VoteCounter
    pub fn new(ncn: &Pubkey, bump: u8) -> Self {
        Self {
            ncn: *ncn,
            count: PodU64::from(0),
            bump,
            reserved: [0; 7],
        }
    }

    /// Get the current count
    pub fn count(&self) -> u64 {
        self.count.into()
    }

    /// Increment the counter by 1
    pub fn increment(&mut self) -> Result<(), ProgramError> {
        let current_count: u64 = self.count.into();
        let new_count = current_count
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        self.count = PodU64::from(new_count);
        Ok(())
    }

    /// Find the program address for the vote counter
    pub fn find_program_address(program_id: &Pubkey, ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = vec![b"vote_counter".to_vec(), ncn.as_ref().to_vec()];
        let (address, bump) =
            Pubkey::find_program_address(&[b"vote_counter", ncn.as_ref()], program_id);
        (address, bump, seeds)
    }

    /// Load and validate the vote counter account
    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_address = Self::find_program_address(program_id, ncn).0;
        check_load(
            program_id,
            account,
            &expected_address,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_vote_counter_creation() {
        let ncn = Pubkey::new_unique();
        let bump = 255;
        let counter = VoteCounter::new(&ncn, bump);

        assert_eq!(counter.ncn, ncn);
        assert_eq!(counter.count(), 0);
        assert_eq!(counter.bump, bump);
    }

    #[test]
    fn test_vote_counter_increment() {
        let ncn = Pubkey::new_unique();
        let mut counter = VoteCounter::new(&ncn, 255);

        assert_eq!(counter.count(), 0);

        counter.increment().unwrap();
        assert_eq!(counter.count(), 1);

        counter.increment().unwrap();
        assert_eq!(counter.count(), 2);
    }

    #[test]
    fn test_vote_counter_size() {
        assert_eq!(VoteCounter::SIZE, 8 + 32 + 8 + 1 + 7);
        assert_eq!(VoteCounter::LEN, 32 + 8 + 1 + 7);
    }
}
