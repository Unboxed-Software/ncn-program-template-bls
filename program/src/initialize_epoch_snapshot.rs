use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{
    account_payer::AccountPayer, constants::MAX_REALLOC_BYTES, epoch_snapshot::EpochSnapshot,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Initializes the epoch snapshot account with minimal size.
/// A subsequent realloc instruction is needed to set the full size and initialize the data.
///
/// ### Parameters:
/// - `epoch`: The target epoch
///
/// ### Accounts:
/// 1. `[]` ncn: The NCN account
/// 2. `[writable]` epoch_snapshot: The epoch snapshot account to initialize
/// 3. `[writable, signer]` account_payer: Account paying for initialization
/// 4. `[]` system_program: Solana System Program
pub fn process_initialize_epoch_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [ncn, epoch_snapshot, account_payer, system_program] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    load_system_account(epoch_snapshot, true)?;
    load_system_program(system_program)?;

    let (epoch_snapshot_pubkey, epoch_snapshot_bump, mut epoch_snapshot_seeds) =
        EpochSnapshot::find_program_address(program_id, ncn.key);
    epoch_snapshot_seeds.push(vec![epoch_snapshot_bump]);

    if epoch_snapshot_pubkey.ne(epoch_snapshot.key) {
        msg!("Error: Incorrect epoch snapshot PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        epoch_snapshot,
        system_program,
        program_id,
        MAX_REALLOC_BYTES as usize,
        &epoch_snapshot_seeds,
    )?;

    Ok(())
}
