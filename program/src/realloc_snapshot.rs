use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{
    account_payer::AccountPayer, config::Config, error::NCNProgramError, snapshot::Snapshot,
    utils::get_new_size,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Reallocates the snapshot account to its full size and initializes the data structure.
///
/// ### Accounts:
/// 1. `[]` ncn: The NCN account
/// 2. `[]` config: The NCN program configuration
/// 3. `[writable]` snapshot: The snapshot account to resize and initialize
/// 4. `[writable, signer]` account_payer: Account paying for reallocation
/// 5. `[]` system_program: Solana System Program
pub fn process_realloc_snapshot(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [ncn, config, snapshot, account_payer, system_program] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Config::load(program_id, config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    let (snapshot_pda, snapshot_bump, _) = Snapshot::find_program_address(program_id, ncn.key);

    if snapshot_pda != *snapshot.key {
        msg!("Error: Snapshot account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    let snapshot_size = snapshot.data_len();
    if snapshot_size < Snapshot::SIZE {
        let new_size = get_new_size(snapshot_size, Snapshot::SIZE)?;
        AccountPayer::pay_and_realloc(program_id, ncn.key, account_payer, snapshot, new_size)?;
    } else {
        msg!("Snapshot size is sufficient, no reallocation needed");
    }

    let should_initialize = snapshot.data_len() >= Snapshot::SIZE
        && snapshot.try_borrow_data()?[0] != Snapshot::DISCRIMINATOR;

    if should_initialize {
        let current_slot = Clock::get()?.slot;

        let operator_count: u64 = {
            let ncn_data = ncn.data.borrow();
            let ncn_account = Ncn::try_from_slice_unchecked(&ncn_data)?;
            ncn_account.operator_count()
        };

        if operator_count == 0 {
            msg!("Error: No operators to snapshot");
            return Err(NCNProgramError::NoOperators.into());
        }

        let minimum_stake = {
            let config_data = config.try_borrow_data()?;
            let config_account = Config::try_from_slice_unchecked(&config_data)?;
            *config_account.minimum_stake()
        };

        let mut snapshot_data = snapshot.try_borrow_mut_data()?;
        snapshot_data[0] = Snapshot::DISCRIMINATOR;
        let snapshot_account = Snapshot::try_from_slice_unchecked_mut(&mut snapshot_data)?;

        msg!(
            "Initializing snapshot with operator count: {}",
            operator_count,
        );

        snapshot_account.initialize(
            ncn.key,
            snapshot_bump,
            current_slot,
            operator_count,
            minimum_stake,
        );
    } else {
        msg!("Snapshot already initialized, skipping initialization");
    }

    Ok(())
}
