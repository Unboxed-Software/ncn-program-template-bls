use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{ncn::Ncn, ncn_operator_state::NcnOperatorState, operator::Operator};
use ncn_program_core::{
    config::Config,
    constants::{G1_COMPRESSED_POINT_SIZE, G2_COMPRESSED_POINT_SIZE},
    error::NCNProgramError,
    g1_point::{G1CompressedPoint, G1Point},
    g2_point::{G2CompressedPoint, G2Point},
    loaders::load_ncn_epoch,
    operator_registry::OperatorRegistry,
    schemes::sha256_normalized::Sha256Normalized,
};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

/// Registers an operator in the operator registry with BLS key verification.
///
/// ### Parameters:
/// - `g1_pubkey`: G1 public key in compressed format (32 bytes)
/// - `g2_pubkey`: G2 public key in compressed format (64 bytes)
/// - `signature`: BLS signature of the G1 pubkey signed by the G2 private key (64 bytes uncompressed G1 point)
///
/// ### Accounts:
/// 1. `[]` config: NCN configuration account
/// 2. `[writable]` operator_registry: The operator registry to update
/// 3. `[]` ncn: The NCN account
/// 4. `[]` operator: The operator to register
/// 5. `[signer]` operator_admin: The operator admin that must sign
/// 6. `[]` ncn_operator_state: The connection between NCN and operator
/// 7. `[]` restaking_config: Restaking configuration account
pub fn process_register_operator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
    g2_pubkey: [u8; G2_COMPRESSED_POINT_SIZE],
    signature: [u8; 64],
) -> ProgramResult {
    let [config, operator_registry, ncn, operator, operator_admin, ncn_operator_state, restaking_config] =
        accounts
    else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Config::load(program_id, config, ncn.key, false)?;
    OperatorRegistry::load(program_id, operator_registry, ncn.key, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    NcnOperatorState::load(
        &jito_restaking_program::id(),
        ncn_operator_state,
        ncn,
        operator,
        false,
    )?;

    // Verify that the operator_admin is authorized to register this operator
    {
        let operator_data = operator.data.borrow();
        let operator_account = Operator::try_from_slice_unchecked(&operator_data)?;

        if operator_account.admin.ne(operator_admin.key) {
            msg!("Error: Operator admin is not authorized to register this operator");
            return Err(ProgramError::InvalidAccountData);
        }

        if !operator_admin.is_signer {
            msg!("Error: Operator admin must sign the transaction");
            return Err(ProgramError::MissingRequiredSignature);
        }
    }

    let current_slot = Clock::get()?.slot;
    let (_, ncn_epoch_length) = load_ncn_epoch(restaking_config, current_slot, None)?;

    let is_active = {
        let ncn_operator_state_data = ncn_operator_state.data.borrow();
        let ncn_operator_state_account =
            NcnOperatorState::try_from_slice_unchecked(&ncn_operator_state_data)?;

        let ncn_operator_okay = ncn_operator_state_account
            .ncn_opt_in_state
            .is_active(current_slot, ncn_epoch_length)?;

        let operator_ncn_okay = ncn_operator_state_account
            .operator_opt_in_state
            .is_active(current_slot, ncn_epoch_length)?;

        ncn_operator_okay && operator_ncn_okay
    };

    if !is_active {
        msg!("Error: Operator <> NCN connection is not acctive");
        return Err(ProgramError::from(
            NCNProgramError::OperatorNcnConnectionNotActive,
        ));
    }

    // Verify BLS signature: signature should be G1 pubkey signed by G2 private key
    {
        // Convert the provided keys to points
        let g1_compressed = G1CompressedPoint::from(g1_pubkey);
        let g2_compressed = G2CompressedPoint::from(g2_pubkey);
        let signature = G1Point::from(signature);

        // Convert to uncompressed points for verification
        let g1_point = G1Point::try_from(&g1_compressed)
            .map_err(|_| NCNProgramError::G1PointDecompressionError)?;
        let g2_point = G2Point::try_from(g2_compressed)
            .map_err(|_| NCNProgramError::G2PointDecompressionError)?;

        // First verify that G1 and G2 are from the same private key
        let keypair_valid = g1_point
            .verify_g2(&g2_point)
            .map_err(|_| NCNProgramError::BLSVerificationError)?;

        if !keypair_valid {
            msg!("Error: G1 and G2 public keys are not from the same private key");
            return Err(ProgramError::from(NCNProgramError::BLSVerificationError));
        }

        // Verify the BLS signature: the signature should be the G1 pubkey signed by the G2 private key
        // The message being signed is the G1 pubkey itself
        g2_point
            .verify_signature::<Sha256Normalized, _, _>(signature, &g1_pubkey)
            .map_err(|_| NCNProgramError::BLSVerificationError)?;

        msg!("BLS signature verification successful");
    }

    let clock = Clock::get()?;
    let slot = clock.slot;

    let mut operator_registry_data = operator_registry.try_borrow_mut_data()?;
    let operator_registry_account =
        OperatorRegistry::try_from_slice_unchecked_mut(&mut operator_registry_data)?;

    let operator_index = {
        let operator_data = operator.data.borrow();
        let operator_account = Operator::try_from_slice_unchecked(&operator_data)?;
        operator_account.index()
    };

    operator_registry_account.register_operator(
        operator.key,
        &g1_pubkey,
        &g2_pubkey,
        operator_index,
        slot,
    )?;

    msg!(
        "Operator registered successfully with index {}",
        operator_index
    );

    Ok(())
}
