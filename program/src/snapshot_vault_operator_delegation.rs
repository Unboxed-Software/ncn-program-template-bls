use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn::Ncn, ncn_vault_ticket::NcnVaultTicket, operator::Operator,
};
use jito_vault_core::{
    vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
};
use ncn_program_core::{
    config::Config as NcnConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::NCNProgramError,
    loaders::load_ncn_epoch,
    stake_weight::StakeWeights,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Records the delegation between a vault and an operator at a specific epoch.
///
/// ### Parameters:
/// - `epoch`: The target epoch
///
/// ### Accounts:
/// 1. `[writable]` epoch_state: The epoch state account for the target epoch
/// 2. `[]` ncn_config: NCN configuration account
/// 3. `[]` restaking_config: Restaking configuration account
/// 4. `[]` ncn: The NCN account
/// 5. `[]` operator: The operator account
/// 6. `[]` vault: The vault account
/// 7. `[]` vault_ncn_ticket: The vault NCN ticket
/// 8. `[]` ncn_vault_ticket: The NCN vault ticket
/// 9. `[]` vault_operator_delegation: The delegation between vault and operator
/// 10. `[]` weight_table: The weight table for the epoch
/// 11. `[writable]` epoch_snapshot: Epoch snapshot account containing operator snapshots
pub fn process_snapshot_vault_operator_delegation(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, restaking_config, ncn, operator, vault, vault_ncn_ticket, ncn_vault_ticket, vault_operator_delegation, weight_table, epoch_snapshot] =
        accounts
    else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    NcnConfig::load(program_id, ncn_config, ncn.key, false)?;
    Config::load(&jito_restaking_program::id(), restaking_config, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    Vault::load(&jito_vault_program::id(), vault, false)?;

    NcnVaultTicket::load(
        &jito_restaking_program::id(),
        ncn_vault_ticket,
        ncn,
        vault,
        false,
    )?;

    if !vault_ncn_ticket.data_is_empty() {
        VaultNcnTicket::load(
            &jito_vault_program::id(),
            vault_ncn_ticket,
            vault,
            ncn,
            false,
        )?;
    }

    if !vault_operator_delegation.data_is_empty() {
        VaultOperatorDelegation::load(
            &jito_vault_program::id(),
            vault_operator_delegation,
            vault,
            operator,
            false,
        )?;
    }

    let current_slot = Clock::get()?.slot;

    let (_, ncn_epoch_length) = load_ncn_epoch(restaking_config, current_slot, None)?;

    WeightTable::load(program_id, weight_table, ncn.key, epoch, false)?;
    EpochSnapshot::load(program_id, epoch_snapshot, ncn.key, true)?;

    // check vault is up to date
    let vault_needs_update = {
        let vault_data = vault.data.borrow();
        let vault_account = Vault::try_from_slice_unchecked(&vault_data)?;

        vault_account.is_update_needed(current_slot, ncn_epoch_length)?
    };
    if vault_needs_update {
        msg!("Error: Vault is not up to date");
        return Err(NCNProgramError::VaultNeedsUpdate.into());
    }

    let (vault_index, st_mint) = {
        let vault_data = vault.data.borrow();
        let vault_account = Vault::try_from_slice_unchecked(&vault_data)?;
        (vault_account.vault_index(), vault_account.supported_mint)
    };

    let mut epoch_snapshot_data = epoch_snapshot.try_borrow_mut_data()?;
    let epoch_snapshot_account =
        EpochSnapshot::try_from_slice_unchecked_mut(&mut epoch_snapshot_data)?;
    let operator_snapshot = *epoch_snapshot_account
        .find_operator_snapshot(operator.key)
        .ok_or_else(|| {
            msg!(
                "Error: Operator snapshot not found for operator: {}",
                operator.key
            );
            NCNProgramError::OperatorIsNotInSnapshot
        })?;

    let mut cloned_operator_snapshot = operator_snapshot.clone();

    if !cloned_operator_snapshot.is_active() {}

    // Check if operator has valid BN128 G1 pubkey and determine active status
    let is_active = {
        if !cloned_operator_snapshot.have_valid_bn128_g1_pubkey() {
            false
        } else {
            let ncn_vault_okay = {
                let ncn_vault_ticket_data = ncn_vault_ticket.data.borrow();
                let ncn_vault_ticket_account =
                    NcnVaultTicket::try_from_slice_unchecked(&ncn_vault_ticket_data)?;

                // If the NCN removes a vault, it should immediately be barred from the snapshot
                ncn_vault_ticket_account
                    .state
                    .is_active(current_slot, ncn_epoch_length)?
            };

            let vault_ncn_okay = {
                if vault_ncn_ticket.data_is_empty() {
                    false
                } else {
                    let vault_ncn_ticket_data = vault_ncn_ticket.data.borrow();
                    let vault_ncn_ticket_account =
                        VaultNcnTicket::try_from_slice_unchecked(&vault_ncn_ticket_data)?;

                    // If a vault removes itself from the ncn, it should still be able to participate
                    // until it is finished cooling down - this is so the operators with delegation
                    // from this vault can still participate
                    vault_ncn_ticket_account
                        .state
                        .is_active_or_cooldown(current_slot, ncn_epoch_length)?
                }
            };

            let delegation_dne = vault_operator_delegation.data_is_empty();

            vault_ncn_okay && ncn_vault_okay && !delegation_dne
        }
    };

    msg!("Vault operator delegation active status: {}", is_active);

    let (total_stake_weight, next_epoch_stake_weight) = {
        let weight_table_data = weight_table.data.borrow();
        let weight_table_account = WeightTable::try_from_slice_unchecked(&weight_table_data)?;

        weight_table_account.check_registry_for_vault(vault_index)?;

        let (total_stake_weight, next_epoch_stake_weight): (u128, u128) = if is_active {
            let vault_operator_delegation_data = vault_operator_delegation.data.borrow();
            let vault_operator_delegation_account =
                VaultOperatorDelegation::try_from_slice_unchecked(&vault_operator_delegation_data)?;

            OperatorSnapshot::calculate_stake_weights(
                vault_operator_delegation_account,
                weight_table_account,
                &st_mint,
            )?
        } else {
            (0u128, 0u128)
        };

        (total_stake_weight, next_epoch_stake_weight)
    };

    // Increment vault operator delegation and check if finalized
    let this_epoch_stake_weight = StakeWeights::snapshot(total_stake_weight)?;
    let next_epoch_stake_weight = StakeWeights::snapshot(next_epoch_stake_weight)?;
    let (ncn_operator_index, is_snapshoted) = {
        let is_snapshoted = cloned_operator_snapshot.is_snapshoted();
        cloned_operator_snapshot.snapshot_vault_operator_delegation(
            current_slot,
            &this_epoch_stake_weight,
            &next_epoch_stake_weight,
            epoch_snapshot_account.minimum_stake_weight(),
        )?;

        let ncn_operator_index = cloned_operator_snapshot.ncn_operator_index();

        (ncn_operator_index, is_snapshoted)
    };

    // If operator is finalized, increment operator registration
    if !is_snapshoted {
        epoch_snapshot_account.increment_operator_registration(current_slot)?;
    }

    epoch_snapshot_account.update_operator_snapshot(
        cloned_operator_snapshot.ncn_operator_index() as usize,
        &cloned_operator_snapshot,
    );

    // Update Epoch State
    {
        msg!("Updating epoch state for vault operator delegation");
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account
            .update_snapshot_vault_operator_delegation(ncn_operator_index as usize)?;
    }

    Ok(())
}
