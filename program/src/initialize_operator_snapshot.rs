use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::{ncn::Ncn, ncn_operator_state::NcnOperatorState, operator::Operator};
use ncn_program_core::{
    account_payer::AccountPayer,
    error::NCNProgramError,
    g1_point::G1CompressedPoint,
    loaders::load_ncn_epoch,
    ncn_operator_account::NCNOperatorAccount,
    snapshot::{OperatorSnapshot, Snapshot},
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Initializes a snapshot for a specific operator, storing their stake weights within the snapshot.
///
/// ### Parameters:
/// - `epoch`: The target epoch
///
/// ### Accounts:
/// 1. `[]` restaking_config: Restaking configuration account
/// 2. `[]` ncn: The NCN account
/// 3. `[]` operator: The operator account to snapshot
/// 4. `[]` ncn_operator_state: The connection between NCN and operator
/// 5. `[]` ncn_operator_account: The ncn operator account PDA containing BLS keys (optional)
/// 6. `[writable]` snapshot: Snapshot account containing operator snapshots
/// 7. `[writable, signer]` account_payer: Account paying for any additional rent
/// 8. `[]` system_program: Solana System Program
pub fn process_initialize_operator_snapshot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [restaking_config, ncn, operator, ncn_operator_state, ncn_operator_account, snapshot, account_payer, system_program] =
        accounts
    else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    NcnOperatorState::load(
        &jito_restaking_program::id(),
        ncn_operator_state,
        ncn,
        operator,
        false,
    )?;
    Snapshot::load(program_id, snapshot, ncn.key, true)?;
    load_system_program(system_program)?;
    AccountPayer::load(program_id, account_payer, ncn.key, true)?;

    // Check if operator index is valid
    let ncn_operator_index = {
        let snapshot_data = snapshot.data.borrow();
        let snapshot = Snapshot::try_from_slice_unchecked(&snapshot_data)?;

        let ncn_operator_state_data = ncn_operator_state.data.borrow();
        let ncn_operator_state =
            NcnOperatorState::try_from_slice_unchecked(&ncn_operator_state_data)?;

        let operator_count = snapshot.operator_count();
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

    if !is_active {
        msg!("NCN <> operator Opt-in state is inactive");
        return Err(NCNProgramError::NCNOperatorOptInInactive.into());
    }

    let operator_index = {
        let operator_data = operator.data.borrow();
        let operator_account = Operator::try_from_slice_unchecked(&operator_data)?;
        operator_account.index()
    };
    msg!("Operator index: {}", operator_index);

    // Try to get the G1 pubkey from the ncn operator account PDA
    let g1_pubkey: Option<[u8; 32]> = {
        // First check if the ncn operator account account exists by verifying it's the correct PDA
        let (expected_ncn_operator_account_pda, _, _) =
            NCNOperatorAccount::find_program_address(program_id, ncn.key, operator.key);

        if *ncn_operator_account.key == expected_ncn_operator_account_pda {
            // The account exists and is the correct PDA, try to load it
            match NCNOperatorAccount::load(
                program_id,
                ncn_operator_account,
                ncn.key,
                operator.key,
                false,
            ) {
                Ok(()) => {
                    let ncn_operator_account_data = ncn_operator_account.try_borrow_data()?;
                    let ncn_operator_account_account =
                        NCNOperatorAccount::try_from_slice_unchecked(&ncn_operator_account_data)?;
                    Some(*ncn_operator_account_account.g1_pubkey())
                }
                Err(_) => {
                    msg!("NCN Operator Account PDA exists but failed to load properly");
                    None
                }
            }
        } else {
            msg!("NCN Operator Account PDA does not exist for this operator");
            None
        }
    };
    msg!("G1 pubkey: {:?}", g1_pubkey);

    // Create operator snapshot and add it to the snapshot
    let operator_snapshot = OperatorSnapshot::new(
        operator.key,
        current_slot,
        is_active,
        ncn_operator_index,
        operator_index,
        g1_pubkey.unwrap_or(G1CompressedPoint::default().0),
    )?;

    let mut snapshot_data = snapshot.try_borrow_mut_data()?;
    let snapshot_account = Snapshot::try_from_slice_unchecked_mut(&mut snapshot_data)?;

    // Add the operator snapshot to the snapshot
    snapshot_account.add_operator_snapshot(operator_snapshot)?;

    if is_active && g1_pubkey.is_some() {
        snapshot_account.register_operator_g1_pubkey(&g1_pubkey.unwrap())?;
    }

    if !is_active {
        // Increment operator registration for an inactive operator
        snapshot_account.increment_operator_registration(current_slot)?;
    }

    Ok(())
}
