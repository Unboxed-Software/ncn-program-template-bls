use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use ncn_program_core::{
    account_payer::AccountPayer, config::Config as NcnConfig, operator_registry::OperatorRegistry,
    utils::get_new_size,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Resizes the operator registry account to accommodate more operators.
///
/// ### Accounts:
/// 1. `[]` config: NCN configuration account
/// 2. `[writable]` operator_registry: The operator registry account to resize
/// 3. `[]` ncn: The NCN account
/// 4. `[writable, signer]` account_payer: Account paying for the reallocation
/// 5. `[]` system_program: Solana System Program
pub fn process_realloc_operator_registry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [ncn_config, operator_registry, ncn, account_payer, system_program] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    NcnConfig::load(program_id, ncn_config, ncn.key, false)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    let (operator_registry_pda, operator_registry_bump, mut operator_registry_seeds) =
        OperatorRegistry::find_program_address(program_id, ncn.key);
    operator_registry_seeds.push(vec![operator_registry_bump]);

    if operator_registry_pda != *operator_registry.key {
        msg!("Error: Operator registry account is not at the correct PDA");
        return Err(ProgramError::InvalidSeeds);
    }

    let operator_registry_size = operator_registry.data_len();
    if operator_registry_size < OperatorRegistry::SIZE {
        let new_size = get_new_size(operator_registry_size, OperatorRegistry::SIZE)?;

        AccountPayer::pay_and_realloc(
            program_id,
            ncn.key,
            account_payer,
            operator_registry,
            new_size,
        )?;
    } else {
        msg!("Operator registry size is sufficient, no reallocation needed");
    }

    let should_initialize = operator_registry.data_len() >= OperatorRegistry::SIZE
        && operator_registry.try_borrow_data()?[0] != OperatorRegistry::DISCRIMINATOR;

    if should_initialize {
        let mut operator_registry_data = operator_registry.try_borrow_mut_data()?;
        operator_registry_data[0] = OperatorRegistry::DISCRIMINATOR;
        let operator_registry_account =
            OperatorRegistry::try_from_slice_unchecked_mut(&mut operator_registry_data)?;
        operator_registry_account.initialize(ncn.key, operator_registry_bump);
    } else {
        msg!("Operator registry already initialized, skipping initialization");
    }

    Ok(())
}
