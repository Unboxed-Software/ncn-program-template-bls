use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::operator::Operator;
use ncn_program_core::{
    config::Config, error::NCNProgramError, ncn_operator_account::NCNOperatorAccount,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Updates an operator's IP address and socket in their individual ncn operator account.
///
/// ### Parameters:
/// - `ip_address`: New IP address (IPv4 format, 16 bytes)
/// - `socket`: New socket (16 bytes)
///
/// ### Accounts:
/// 1. `[]` config: NCN configuration account
/// 2. `[writable]` ncn_operator_account: The ncn operator account PDA account to update
/// 3. `[]` ncn: The NCN account
/// 4. `[]` operator: The operator to update
/// 5. `[signer]` operator_admin: The operator admin that must sign
pub fn process_update_operator_ip_socket(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ip_address: [u8; 16],
    socket: [u8; 16],
) -> ProgramResult {
    let [config, ncn_operator_account, ncn, operator, operator_admin] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Config::load(program_id, config, ncn.key, false)?;
    NCNOperatorAccount::load(
        program_id,
        ncn_operator_account,
        ncn.key,
        operator.key,
        true,
    )?;

    // Note: We don't load NCN and operator here since they're not directly accessed
    // but we validate them through the NCNOperatorAccount load which checks the PDA derivation

    // Verify that the operator_admin is authorized to update this operator
    {
        let operator_data = operator.data.borrow();
        let operator_account = Operator::try_from_slice_unchecked(&operator_data)?;

        if operator_account.admin.ne(operator_admin.key) {
            msg!("Error: Operator admin is not authorized to update this operator");
            return Err(ProgramError::InvalidAccountData);
        }

        if !operator_admin.is_signer {
            msg!("Error: Operator admin must sign the transaction");
            return Err(ProgramError::MissingRequiredSignature);
        }
    }

    // Verify that the ncn operator account exists and belongs to the right operator
    {
        let ncn_operator_account_data = ncn_operator_account.try_borrow_data()?;
        let ncn_operator_account_account =
            NCNOperatorAccount::try_from_slice_unchecked(&ncn_operator_account_data)?;

        if ncn_operator_account_account
            .operator_pubkey()
            .ne(operator.key)
        {
            msg!("Error: NCN operator account does not belong to the specified operator");
            return Err(ProgramError::InvalidAccountData);
        }

        if ncn_operator_account_account.ncn().ne(ncn.key) {
            msg!("Error: NCN operator account does not belong to the specified NCN");
            return Err(ProgramError::InvalidAccountData);
        }

        if ncn_operator_account_account.is_empty() {
            msg!("Error: NCN operator account is not initialized");
            return Err(NCNProgramError::NCNOperatorAccountDosentExist.into());
        }
    }

    let mut ncn_operator_account_data = ncn_operator_account.try_borrow_mut_data()?;
    let ncn_operator_account_account =
        NCNOperatorAccount::try_from_slice_unchecked_mut(&mut ncn_operator_account_data)?;

    // Update the operator's IP address and socket
    ncn_operator_account_account.update_ip_socket(ip_address, socket)?;

    msg!(
        "Operator IP address and socket updated successfully for operator {}",
        operator.key
    );

    Ok(())
}
