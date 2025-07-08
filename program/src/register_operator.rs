use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use ncn_program_core::{
    config::Config,
    constants::{G1_COMPRESSED_POINT_SIZE, G2_COMPRESSED_POINT_SIZE},
    error::NCNProgramError,
    g1_point::{G1CompressedPoint, G1Point},
    g2_point::{G2CompressedPoint, G2Point},
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
/// - `signature`: BLS signature of the G1 pubkey signed by the G2 private key (32 bytes)
///
/// ### Accounts:
/// 1. `[]` config: NCN configuration account
/// 2. `[writable]` operator_registry: The operator registry to update
/// 3. `[]` ncn: The NCN account
/// 4. `[]` operator: The operator to register
/// 5. `[signer]` operator_admin: The operator admin that must sign
pub fn process_register_operator(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    g1_pubkey: [u8; G1_COMPRESSED_POINT_SIZE],
    g2_pubkey: [u8; G2_COMPRESSED_POINT_SIZE],
    signature: [u8; 64],
) -> ProgramResult {
    let [config, operator_registry, ncn, operator, operator_admin] = accounts else {
        msg!("Error: Not enough account keys provided");
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Config::load(program_id, config, ncn.key, false)?;
    OperatorRegistry::load(program_id, operator_registry, ncn.key, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;

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
