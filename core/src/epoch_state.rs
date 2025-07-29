use core::fmt;
use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodBool, PodU64},
    AccountDeserialize, Discriminator,
};
use shank::{ShankAccount, ShankType};
use solana_program::{
    account_info::AccountInfo, epoch_schedule::EpochSchedule, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    constants::MAX_OPERATORS, discriminators::Discriminators, error::NCNProgramError,
    loaders::check_load,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AccountStatus {
    DNE = 0,
    Created = 1,
    CreatedWithReceiver = 2,
    Closed = 3,
}

#[derive(Debug, Default, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct EpochAccountStatus {
    epoch_state: u8,
    weight_table: u8,
}

impl EpochAccountStatus {
    pub const SIZE: usize = size_of::<Self>();

    pub const fn get_account_status(u: u8) -> Result<AccountStatus, NCNProgramError> {
        match u {
            0 => Ok(AccountStatus::DNE),
            1 => Ok(AccountStatus::Created),
            2 => Ok(AccountStatus::CreatedWithReceiver),
            3 => Ok(AccountStatus::Closed),
            _ => Err(NCNProgramError::InvalidAccountStatus),
        }
    }

    pub const fn epoch_state(&self) -> Result<AccountStatus, NCNProgramError> {
        Self::get_account_status(self.epoch_state)
    }

    pub const fn weight_table(&self) -> Result<AccountStatus, NCNProgramError> {
        Self::get_account_status(self.weight_table)
    }

    pub fn set_epoch_state(&mut self, status: AccountStatus) {
        self.epoch_state = status as u8;
    }

    pub fn set_weight_table(&mut self, status: AccountStatus) {
        self.weight_table = status as u8;
    }

    pub fn are_all_closed(&self) -> bool {
        // We don't need to check epoch state since it's the account we are closing

        if self.weight_table != AccountStatus::Closed as u8 {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct Progress {
    /// tally
    tally: PodU64,
    /// total
    total: PodU64,
}

impl Default for Progress {
    fn default() -> Self {
        Self {
            tally: PodU64::from(Self::INVALID),
            total: PodU64::from(Self::INVALID),
        }
    }
}

impl Progress {
    pub const INVALID: u64 = u64::MAX;
    pub const SIZE: usize = size_of::<Self>();

    pub fn new(total: u64) -> Self {
        Self {
            tally: PodU64::from(0),
            total: PodU64::from(total),
        }
    }

    pub fn tally(&self) -> u64 {
        self.tally.into()
    }

    pub fn total(&self) -> u64 {
        self.total.into()
    }

    pub fn increment_one(&mut self) -> Result<(), NCNProgramError> {
        self.increment(1)
    }

    pub fn mark_complete(&mut self) {
        let total = self.total.into();
        self.set_tally(total);
    }

    pub fn increment(&mut self, amount: u64) -> Result<(), NCNProgramError> {
        self.tally = PodU64::from(
            self.tally()
                .checked_add(amount)
                .ok_or(NCNProgramError::ArithmeticOverflow)?,
        );

        Ok(())
    }

    pub fn set_tally(&mut self, tally: u64) {
        self.tally = PodU64::from(tally);
    }

    pub fn set_total(&mut self, total: u64) {
        self.total = PodU64::from(total);
    }

    pub fn is_invalid(&self) -> bool {
        self.tally.eq(&PodU64::from(Self::INVALID)) || self.total.eq(&PodU64::from(Self::INVALID))
    }

    pub fn is_complete(&self) -> bool {
        if self.is_invalid() {
            false
        } else {
            self.tally() >= self.total()
        }
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochState {
    /// The NCN this snapshot is for
    ncn: Pubkey,
    /// The epoch this snapshot is for
    epoch: PodU64,
    /// The bump seed for the PDA
    pub bump: u8,

    /// The time this snapshot was created
    slot_created: PodU64,

    /// The number of operators
    // TODO: check if we need this
    operator_count: PodU64,

    /// The number of vaults
    vault_count: PodU64,

    /// All of the epoch accounts status
    account_status: EpochAccountStatus,

    /// Progress on weight set
    set_weight_progress: Progress,

    /// Progress on Snapshotting Operators
    operator_snapshot_progress: [Progress; 256],

    /// Is closing
    is_closing: PodBool,
}

impl Discriminator for EpochState {
    const DISCRIMINATOR: u8 = Discriminators::EpochState as u8;
}

impl EpochState {
    const EPOCH_STATE_SEED: &'static [u8] = b"epoch_state";
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: &Pubkey, epoch: u64, bump: u8, slot_created: u64) -> Self {
        Self {
            ncn: *ncn,
            epoch: PodU64::from(epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            operator_count: PodU64::from(0),
            vault_count: PodU64::from(0),
            account_status: EpochAccountStatus::default(),
            set_weight_progress: Progress::default(),
            operator_snapshot_progress: [Progress::default(); MAX_OPERATORS],
            is_closing: PodBool::from(false),
        }
    }

    pub fn initialize(&mut self, ncn: &Pubkey, epoch: u64, bump: u8, slot_created: u64) {
        // Initializes field by field to avoid overflowing stack
        self.ncn = *ncn;
        self.bump = bump;
        self.epoch = PodU64::from(epoch);
        self.slot_created = PodU64::from(slot_created);
    }

    pub fn seeds(ncn: &Pubkey, epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                Self::EPOCH_STATE_SEED.to_vec(),
                ncn.to_bytes().to_vec(),
                epoch.to_le_bytes().to_vec(),
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
        let (address, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (address, bump, seeds)
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
        account_to_close: &Self,
        ncn: &Pubkey,
        epoch: u64,
    ) -> Result<(), ProgramError> {
        if account_to_close.ncn().ne(ncn) {
            msg!("Epoch State NCN does not match NCN");
            return Err(NCNProgramError::CannotCloseAccount.into());
        }

        if account_to_close.epoch().ne(&epoch) {
            msg!("Epoch State epoch does not match epoch");
            return Err(NCNProgramError::CannotCloseAccount.into());
        }

        // Check all other accounts are closed
        if !account_to_close.account_status.are_all_closed() {
            msg!("Cannot close Epoch State until all other accounts are closed");
            return Err(NCNProgramError::CannotCloseEpochStateAccount.into());
        }

        Ok(())
    }

    pub fn load_and_check_is_closing(
        program_id: &Pubkey,
        account: &AccountInfo,
        ncn: &Pubkey,
        epoch: u64,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let account_data = account.try_borrow_data()?;
        let account_struct = Self::try_from_slice_unchecked(&account_data)?;

        if account_struct.is_closing() {
            msg!("Epoch is closing down");
            return Err(NCNProgramError::EpochIsClosingDown.into());
        }

        Self::load(program_id, account, ncn, epoch, expect_writable)
    }

    // ------------ GETTERS ------------

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn slot_created(&self) -> u64 {
        self.slot_created.into()
    }

    pub fn is_closing(&self) -> bool {
        self.is_closing.into()
    }

    pub fn operator_count(&self) -> u64 {
        self.operator_count.into()
    }

    pub fn vault_count(&self) -> u64 {
        self.vault_count.into()
    }

    pub const fn account_status(&self) -> &EpochAccountStatus {
        &self.account_status
    }

    pub const fn set_weight_progress(&self) -> Progress {
        self.set_weight_progress
    }

    pub const fn operator_snapshot_progress(&self, ncn_operator_index: usize) -> Progress {
        self.operator_snapshot_progress[ncn_operator_index]
    }

    // ------------ UPDATERS ------------
    pub fn update_realloc_epoch_state(&mut self) {
        self.account_status.set_epoch_state(AccountStatus::Created);
    }

    pub fn update_realloc_weight_table(&mut self, vault_count: u64, st_mint_count: u64) {
        self.account_status.set_weight_table(AccountStatus::Created);

        self.vault_count = PodU64::from(vault_count);
        self.set_weight_progress = Progress::new(st_mint_count);
    }

    pub fn update_set_weight(&mut self, weights_set: u64, st_mint_count: u64) {
        self.set_weight_progress.set_tally(weights_set);
        self.set_weight_progress.set_total(st_mint_count)
    }

    pub fn update_snapshot_vault_operator_delegation(
        &mut self,
        ncn_operator_index: usize,
    ) -> Result<(), NCNProgramError> {
        self.operator_snapshot_progress[ncn_operator_index].mark_complete();

        Ok(())
    }

    // Ballot box functionality removed

    pub fn update_cast_vote(
        &mut self,
        _operators_voted: u64,
        _current_slot: u64,
    ) -> Result<(), NCNProgramError> {
        Ok(())
    }

    // ---------- CLOSERS ----------
    pub fn set_is_closing(&mut self) {
        self.is_closing = PodBool::from(true);
    }

    pub fn close_epoch_state(&mut self) {
        self.account_status.set_epoch_state(AccountStatus::Closed);
    }

    pub fn close_weight_table(&mut self) {
        self.account_status.set_weight_table(AccountStatus::Closed);
    }

    // ------------ STATE ------------

    pub fn can_close_epoch_accounts(
        &self,
        epoch_schedule: &EpochSchedule,
        epochs_after_consensus_before_close: u64,
        current_slot: u64,
    ) -> Result<bool, ProgramError> {
        let current_epoch = epoch_schedule.get_epoch(current_slot);
        let epoch_delta = current_epoch.saturating_sub(self.epoch());
        let can_close_epoch_accounts = epoch_delta >= epochs_after_consensus_before_close;
        Ok(can_close_epoch_accounts)
    }

    pub fn current_state(
        &self,
        epoch_schedule: &EpochSchedule,
        _valid_slots_after_consensus: u64,
        epochs_after_consensus_before_close: u64,
        current_slot: u64,
    ) -> Result<State, ProgramError> {
        if self.can_close_epoch_accounts(
            epoch_schedule,
            epochs_after_consensus_before_close,
            current_slot,
        ) == Ok(true)
        {
            return Ok(State::Close);
        }

        if self.account_status.weight_table()? == AccountStatus::DNE
            || !self.set_weight_progress.is_complete()
        {
            return Ok(State::SetWeight);
        }

        Ok(State::PostVoteCooldown)
    }

    pub fn current_state_patched(
        &self,
        epoch_schedule: &EpochSchedule,
        _valid_slots_after_consensus: u64,
        epochs_after_consensus_before_close: u64,
        st_mint_count: u64,
        current_slot: u64,
    ) -> Result<State, ProgramError> {
        if self.can_close_epoch_accounts(
            epoch_schedule,
            epochs_after_consensus_before_close,
            current_slot,
        ) == Ok(true)
        {
            return Ok(State::Close);
        }

        if self.account_status.weight_table()? == AccountStatus::DNE
            || self.set_weight_progress.tally() < st_mint_count
        {
            return Ok(State::SetWeight);
        }

        Ok(State::PostVoteCooldown)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum State {
    SetWeight,
    Snapshot,
    Vote,
    PostVoteCooldown,
    Close,
}

#[rustfmt::skip]
impl fmt::Display for EpochState {
   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
       writeln!(f, "\n\n----------- Epoch State -------------")?;
       writeln!(f, "  NCN:                          {}", self.ncn)?;
       writeln!(f, "  Epoch:                        {}", self.epoch())?;
       writeln!(f, "  Bump:                         {}", self.bump)?;
       writeln!(f, "  Slot Created:                 {}", self.slot_created())?;
       writeln!(f, "  Operator Count:               {}", self.operator_count())?;
       writeln!(f, "  Vault Count:                  {}", self.vault_count())?;

       writeln!(f, "\nAccount Status:")?;
       writeln!(f, "  Epoch State:                  {:?}", self.account_status.epoch_state().unwrap())?;
       writeln!(f, "  Weight Table:                 {:?}", self.account_status.weight_table().unwrap())?;
       


       writeln!(f, "\nProgress:")?;
       writeln!(f, "  Set Weight Progress:          {}/{}", self.set_weight_progress.tally(), self.set_weight_progress.total())?;
       
       writeln!(f, "\nOperator Snapshot Progress:")?;
       for i in 0..MAX_OPERATORS {
            if self.operator_snapshot_progress(i).total() > 0 {
                writeln!(f, "  Operator {}:                   {}/{}", i, self.operator_snapshot_progress(i).tally(), self.operator_snapshot_progress(i).total())?;                
            }
       }

       writeln!(f, "\n")?;
       Ok(())
   }
}
