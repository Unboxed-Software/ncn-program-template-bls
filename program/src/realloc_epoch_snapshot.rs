use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{
    account_payer::AccountPayer, config::Config, epoch_snapshot::EpochSnapshot,
    epoch_state::EpochState, error::NCNProgramError, utils::get_new_size,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Reallocates the epoch snapshot account to its full size and initializes the data structure.
///
/// ### Parameters:
/// - `epoch`: The target epoch
///
/// ### Accounts:
/// 1. `[writable]` epoch_state: The epoch state account for the target epoch
/// 2. `[]` ncn: The NCN account
/// 3. `[writable]` epoch_snapshot: The epoch snapshot account to resize and initialize
/// 4. `[writable, signer]` account_payer: Account paying for reallocation
/// 5. `[]` system_program: Solana System Program
pub fn process_realloc_epoch_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn, config, epoch_snapshot, account_payer, system_program] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Config::load(program_id, config, ncn.key, false)?;
    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    let (epoch_snapshot_pda, epoch_snapshot_bump, _) =
        EpochSnapshot::find_program_address(program_id, ncn.key);

    if epoch_snapshot_pda != *epoch_snapshot.key {
        msg!("Error: Epoch snapshot account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    let epoch_snapshot_size = epoch_snapshot.data_len();
    if epoch_snapshot_size < EpochSnapshot::SIZE {
        let new_size = get_new_size(epoch_snapshot_size, EpochSnapshot::SIZE)?;
        AccountPayer::pay_and_realloc(
            program_id,
            ncn.key,
            account_payer,
            epoch_snapshot,
            new_size,
        )?;
    } else {
        msg!("Epoch snapshot size is sufficient, no reallocation needed");
    }

    let should_initialize = epoch_snapshot.data_len() >= EpochSnapshot::SIZE
        && epoch_snapshot.try_borrow_data()?[0] != EpochSnapshot::DISCRIMINATOR;

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

        let mut epoch_snapshot_data = epoch_snapshot.try_borrow_mut_data()?;
        epoch_snapshot_data[0] = EpochSnapshot::DISCRIMINATOR;
        let epoch_snapshot_account =
            EpochSnapshot::try_from_slice_unchecked_mut(&mut epoch_snapshot_data)?;

        msg!(
            "Initializing epoch snapshot with operator count: {}",
            operator_count,
        );

        epoch_snapshot_account.initialize(
            ncn.key,
            epoch,
            epoch_snapshot_bump,
            current_slot,
            operator_count,
            minimum_stake,
        );
    } else {
        msg!("Epoch snapshot already initialized, skipping initialization");
    }

    Ok(())
}
