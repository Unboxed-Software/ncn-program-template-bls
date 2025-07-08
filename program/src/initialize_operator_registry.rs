use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{
    account_payer::AccountPayer, config::Config as NcnConfig, constants::MAX_REALLOC_BYTES,
    operator_registry::OperatorRegistry,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Initializes the operator registry for tracking operators and their BLS public keys.
///
/// ### Accounts:
/// 1. `[]` config: NCN configuration account
/// 2. `[writable]` operator_registry: The operator registry account to initialize
/// 3. `[]` ncn: The NCN account
/// 4. `[writable, signer]` account_payer: Account paying for the initialization
/// 5. `[]` system_program: Solana System Program
pub fn process_initialize_operator_registry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [ncn_config, operator_registry, ncn, account_payer, system_program] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_account(operator_registry, true)?;
    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    NcnConfig::load(program_id, ncn_config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    let (operator_registry_pda, operator_registry_bump, mut operator_registry_seeds) =
        OperatorRegistry::find_program_address(program_id, ncn.key);
    operator_registry_seeds.push(vec![operator_registry_bump]);

    if operator_registry_pda != *operator_registry.key {
        msg!("Error: Invalid operator registry PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        operator_registry,
        system_program,
        program_id,
        MAX_REALLOC_BYTES as usize,
        &operator_registry_seeds,
    )?;

    Ok(())
}
