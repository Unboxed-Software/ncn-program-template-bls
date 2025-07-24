use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::{ncn::Ncn, ncn_operator_state::NcnOperatorState, operator::Operator};
use ncn_program_core::{
    account_payer::AccountPayer,
    epoch_marker::EpochMarker,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::NCNProgramError,
    g1_point::G1CompressedPoint,
    loaders::load_ncn_epoch,
    operator_registry::OperatorRegistry,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Initializes a snapshot for a specific operator, storing their stake weights within the epoch snapshot.
///
/// ### Parameters:
/// - `epoch`: The target epoch
///
/// ### Accounts:
/// 1. `[]` epoch_marker: Marker account to prevent duplicate initialization
/// 2. `[writable]` epoch_state: The epoch state account for the target epoch
/// 3. `[]` restaking_config: Restaking configuration account
/// 4. `[]` ncn: The NCN account
/// 5. `[]` operator: The operator account to snapshot
/// 6. `[]` ncn_operator_state: The connection between NCN and operator
/// 7. `[]` operator_registry: The operator registry containing G1 pubkeys
/// 8. `[writable]` epoch_snapshot: Epoch snapshot account containing operator snapshots
/// 9. `[writable, signer]` account_payer: Account paying for any additional rent
/// 10. `[]` system_program: Solana System Program
pub fn process_initialize_operator_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_marker, epoch_state, restaking_config, ncn, operator, ncn_operator_state, operator_registry, epoch_snapshot, account_payer, system_program] =
        accounts
    else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load_and_check_is_closing(program_id, epoch_state, ncn.key, epoch, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    NcnOperatorState::load(
        &jito_restaking_program::id(),
        ncn_operator_state,
        ncn,
        operator,
        false,
    )?;
    EpochSnapshot::load(program_id, epoch_snapshot, ncn.key, epoch, true)?;
    OperatorRegistry::load(program_id, operator_registry, ncn.key, false)?;
    load_system_program(system_program)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;
    EpochMarker::check_dne(program_id, epoch_marker, ncn.key, epoch)?;

    // Check if operator index is valid
    let ncn_operator_index = {
        let epoch_snapshot_data = epoch_snapshot.data.borrow();
        let epoch_snapshot = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

        let ncn_operator_state_data = ncn_operator_state.data.borrow();
        let ncn_operator_state =
            NcnOperatorState::try_from_slice_unchecked(&ncn_operator_state_data)?;

        let operator_count = epoch_snapshot.operator_count();
        let operator_index = ncn_operator_state.index();

        if operator_index >= operator_count {
            msg!(
                "Error: Operator index is out of bounds. Index: {}, Count: {}",
                operator_index,
                operator_count
            );
            return Err(NCNProgramError::OperatorIsNotInSnapshot.into());
        }

        operator_index
    };

    let current_slot = Clock::get()?.slot;

    let (_, ncn_epoch_length) = load_ncn_epoch(restaking_config, current_slot, None)?;

    let is_active = {
        let ncn_operator_state_data = ncn_operator_state.data.borrow();
        let ncn_operator_state_account =
            NcnOperatorState::try_from_slice_unchecked(&ncn_operator_state_data)?;

        // If the NCN removes an operator, it should immediately be barred from the snapshot
        let ncn_operator_okay = ncn_operator_state_account
            .ncn_opt_in_state
            .is_active(current_slot, ncn_epoch_length)?;
        msg!("NCN operator opt-in state active: {}", ncn_operator_okay);

        // If the operator removes itself from the ncn, it should still be able to participate
        // while it is cooling down
        let operator_ncn_okay = ncn_operator_state_account
            .operator_opt_in_state
            .is_active_or_cooldown(current_slot, ncn_epoch_length)?;

        ncn_operator_okay && operator_ncn_okay
    };
    msg!("Operator is active: {}", is_active);

    let operator_index = {
        let operator_data = operator.data.borrow();
        let operator_account = Operator::try_from_slice_unchecked(&operator_data)?;
        operator_account.index()
    };
    msg!("Operator index: {}", operator_index);

    // Get the G1 pubkey from the operator registry
    let g1_pubkey: Option<[u8; 32]> = {
        let operator_registry_data = operator_registry.try_borrow_data()?;
        let operator_registry_account =
            OperatorRegistry::try_from_slice_unchecked(&operator_registry_data)?;
        let operator_entry = operator_registry_account.try_get_operator_entry(operator.key);

        if let Some(operator_entry) = operator_entry {
            Some(*operator_entry.g1_pubkey())
        } else {
            None
        }
    };
    msg!("G1 pubkey: {:?}", g1_pubkey);

    // Create operator snapshot and add it to the epoch snapshot
    let operator_snapshot = OperatorSnapshot::new(
        operator.key,
        current_slot,
        is_active,
        ncn_operator_index,
        operator_index,
        g1_pubkey.unwrap_or(G1CompressedPoint::default().0),
    )?;

    let mut epoch_snapshot_data = epoch_snapshot.try_borrow_mut_data()?;
    let epoch_snapshot_account =
        EpochSnapshot::try_from_slice_unchecked_mut(&mut epoch_snapshot_data)?;

    // Add the operator snapshot to the epoch snapshot
    epoch_snapshot_account.add_operator_snapshot(operator_snapshot)?;

    if is_active && g1_pubkey.is_some() {
        epoch_snapshot_account.register_operator_g1_pubkey(&g1_pubkey.unwrap())?;
    }

    if !is_active {
        // Increment operator registration for an inactive operator
        epoch_snapshot_account.increment_operator_registration(current_slot)?;
    }

    // Update Epoch State
    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account
            .update_initialize_operator_snapshot(ncn_operator_index as usize, is_active)?;
    }

    Ok(())
}
